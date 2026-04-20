# Contributing

## Prerequisites

- Rust 1.75.0+ (install via [rustup](https://rustup.rs/))
- Python 3.12 or 3.13 (**not** 3.14+; PyO3 0.21 in `rust_engine/Cargo.toml` does not support Python 3.14)
- [Poetry](https://python-poetry.org/)

## Setup

```bash
git clone --recursive https://github.com/alihaskar/gap-mm
cd gap-mm
poetry install --with dev
cd rust_engine && poetry run maturin develop --release && cd ..
```

## Running tests

```bash
# All Python tests
poetry run pytest tests/ -v

# Specific suite
poetry run pytest tests/unit/ -v
poetry run pytest tests/integration/ -v

# Rust unit tests (includes bybit.rs gap-probability tests)
cd rust_engine && cargo test
```

## Lint / format

```bash
# Python
poetry run ruff check src/ tests/
poetry run ruff format src/ tests/

# Rust
cd rust_engine
cargo fmt
cargo clippy -- -D warnings
```

## After editing Rust code

Always rebuild before running Python:

```bash
cd rust_engine && poetry run maturin develop --release && cd ..
```

## Commit style

Use conventional commits:

```
feat: add ETH/USDT tick-size preset
fix: correct gap scan off-by-one on bid side
docs: update README architecture diagram
test: add edge case for zero liquidity in gap
```

## Pull requests

1. Fork, create a feature branch off `main`.
2. Keep changes focused — one concern per PR.
3. Ensure `pytest` and `cargo test` pass cleanly.
4. Ensure `ruff check` and `cargo clippy` produce no errors.
5. Update the relevant section of `CHANGELOG.md`.
