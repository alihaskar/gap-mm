"""
gap-mm internal latency benchmark.

Run:
    poetry run python tests/benchmarks/bench_latency.py

Measures three segments of the tick-to-order pipeline:

  [1] Numba signal path
        encode_signal() + calculate_quotes_fast()
        Pure JIT math, no I/O.

  [2] Python tick dispatch  (no I/O)
        Dict unpack + encode_signal + calculate_quotes_fast + price-change guard.

  [3] reconcile() roundtrip  (localhost mock HTTP)
        Python → Rust FFI → reqwest keep-alive → instant ThreadingHTTPServer
        → JSON parse → Python dict.
        Lower bound for a co-located exchange.

  [4] Full pipeline  [1]+[2]+[3] under a single timestamp.
"""

import json
import socket
import statistics
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# ── import gap_mm ─────────────────────────────────────────────────────────────
try:
    from gap_mm.engine import calculate_quotes_fast, encode_signal
except ImportError:
    sys.exit("ERROR: gap_mm not found. Run from the project root with `poetry run python ...`")

try:
    from rust_engine import ExecutionNode
except ImportError:
    sys.exit("ERROR: rust_engine not built. Run `poetry run maturin develop --manifest-path rust_engine/Cargo.toml`")

# ── constants ─────────────────────────────────────────────────────────────────
MID  = 90_000.0
TICK = 0.10

N_WARMUP      =   500
N_BENCH_NUMBA = 5_000
N_BENCH_HTTP  =   500   # each call → up to 2 HTTP round-trips

SUBMIT_BODY = json.dumps({
    "retCode": 0, "retMsg": "OK",
    "result": {"orderId": "BENCH-001", "orderLinkId": "link-001"},
}).encode()


# ── mock HTTP server ──────────────────────────────────────────────────────────

class _Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        # drain body
        length = int(self.headers.get("Content-Length", 0))
        if length:
            self.rfile.read(length)
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(SUBMIT_BODY)))
        self.send_header("Connection", "keep-alive")
        self.end_headers()
        self.wfile.write(SUBMIT_BODY)

    def log_message(self, *_):  # silence access log
        pass


def _start_mock_server() -> tuple[ThreadingHTTPServer, str]:
    server = ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
    port = server.server_address[1]
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    return server, f"http://127.0.0.1:{port}"


# ── stats / reporting ─────────────────────────────────────────────────────────

def _stats(samples_ns: list[int]) -> dict:
    s = sorted(samples_ns)
    n = len(s)
    return {
        "n":    n,
        "min":  s[0],
        "p50":  s[n // 2],
        "p95":  s[int(n * 0.95)],
        "p99":  s[int(n * 0.99)],
        "max":  s[-1],
        "mean": int(statistics.mean(s)),
    }


def _report(name: str, st: dict) -> None:
    def fmt(ns: int) -> str:
        if ns < 1_000:
            return f"{ns} ns"
        if ns < 1_000_000:
            return f"{ns / 1_000:.2f} µs"
        return f"{ns / 1_000_000:.3f} ms"

    print(f"\n{'─' * 58}")
    print(f"  {name}")
    print(f"  n={st['n']:,}")
    print(f"{'─' * 58}")
    print(f"  min   {fmt(st['min'])}")
    print(f"  p50   {fmt(st['p50'])}")
    print(f"  p95   {fmt(st['p95'])}")
    print(f"  p99   {fmt(st['p99'])}")
    print(f"  max   {fmt(st['max'])}")
    print(f"  mean  {fmt(st['mean'])}")


# ── warmup ────────────────────────────────────────────────────────────────────

def _warmup():
    print(f"Warming up Numba ({N_WARMUP} iters)...", end=" ", flush=True)
    for _ in range(N_WARMUP):
        sig, conf = encode_signal(0.82)
        calculate_quotes_fast(MID, sig, conf, TICK)
    print("done")


# ── segment 1 ─────────────────────────────────────────────────────────────────

def bench_segment1():
    samples = []
    for i in range(N_BENCH_NUMBA):
        score = 0.82 if i % 2 == 0 else 0.18
        t0 = time.perf_counter_ns()
        sig, conf = encode_signal(score)
        calculate_quotes_fast(MID, sig, conf, TICK)
        samples.append(time.perf_counter_ns() - t0)
    _report("Segment 1 — encode_signal + calculate_quotes_fast  (Numba JIT)", _stats(samples))


# ── segment 2 ─────────────────────────────────────────────────────────────────

def bench_segment2():
    last_bid: float | None = None
    last_ask: float | None = None

    def _should_update(b: float, a: float) -> bool:
        nonlocal last_bid, last_ask
        if last_bid is None:
            last_bid, last_ask = b, a
            return True
        half = TICK / 2.0
        changed = abs(b - last_bid) > half or abs(a - last_ask) > half
        if changed:
            last_bid, last_ask = b, a
        return changed

    tick_a = {"mid_price": 90_000.0, "gap_prob_resistance_up": 0.82}
    tick_b = {"mid_price": 89_999.0, "gap_prob_resistance_up": 0.18}

    samples = []
    for i in range(N_BENCH_NUMBA):
        data = tick_a if i % 2 == 0 else tick_b
        t0 = time.perf_counter_ns()
        sig, conf = encode_signal(data["gap_prob_resistance_up"])
        mm_bid, mm_ask, *_ = calculate_quotes_fast(data["mid_price"], sig, conf, TICK)
        _should_update(mm_bid, mm_ask)
        samples.append(time.perf_counter_ns() - t0)

    _report("Segment 2 — Python tick dispatch  (no HTTP)", _stats(samples))


# ── segment 3 ─────────────────────────────────────────────────────────────────

def bench_segment3(base_url: str):
    node = ExecutionNode(
        api_key="bench-key",
        api_secret="bench-secret",
        symbol="BTCUSDT",
        market_type="spot",
        tick_size=TICK,
        max_position=1_000.0,
        min_order_size=0.001,
        api_base_url=base_url,
    )

    # warm up the HTTP connection pool (first call opens the socket)
    node.reconcile(target_bid=89_000.0, target_ask=89_010.0)

    samples = []
    for i in range(N_BENCH_HTTP):
        # Cycle through 50 distinct price levels → always submit or amend
        bid = 89_000.0 + (i % 50) * TICK
        ask = 89_010.0 + (i % 50) * TICK
        t0 = time.perf_counter_ns()
        node.reconcile(target_bid=bid, target_ask=ask)
        samples.append(time.perf_counter_ns() - t0)

    _report("Segment 3 — reconcile() roundtrip  (localhost mock HTTP)", _stats(samples))
    return node


# ── full pipeline ─────────────────────────────────────────────────────────────

def bench_full_pipeline(base_url: str):
    node = ExecutionNode(
        api_key="bench-key",
        api_secret="bench-secret",
        symbol="BTCUSDT",
        market_type="spot",
        tick_size=TICK,
        max_position=1_000.0,
        min_order_size=0.001,
        api_base_url=base_url,
    )
    node.reconcile(target_bid=89_000.0, target_ask=89_010.0)  # warm up

    samples = []
    for i in range(N_BENCH_HTTP):
        mid   = MID + (i % 100) * TICK
        score = 0.82 if i % 2 == 0 else 0.18
        t0 = time.perf_counter_ns()
        sig, conf = encode_signal(score)
        mm_bid, mm_ask, *_ = calculate_quotes_fast(mid, sig, conf, TICK)
        node.reconcile(target_bid=mm_bid, target_ask=mm_ask)
        samples.append(time.perf_counter_ns() - t0)

    _report("Full pipeline — tick dict → reconcile() returns", _stats(samples))


# ── main ──────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    print("=" * 58)
    print("  gap-mm latency benchmark")
    print("=" * 58)

    _warmup()
    bench_segment1()
    bench_segment2()

    print(f"\nStarting localhost mock HTTP server...", end=" ", flush=True)
    server, base_url = _start_mock_server()
    print(f"listening on {base_url}")

    bench_segment3(base_url)
    bench_full_pipeline(base_url)

    server.shutdown()
    print("\nDone.")
