# Rust OMS/EMS Execution Logic & Architecture

## High-Level Architecture: The "Brain" vs. The "Hands"

We split the system into two distinct layers within the Rust engine:
1.  **The Strategy (Brain):** Calculates *ideal* prices (already exists in the gap predictor).
2.  **The OMS/EMS (Hands):** Takes those ideal prices and manages the dirty work of talking to the exchange.

## 1. State Management (The "Truth")

We employ a **Dual-State** architecture to handle the race conditions between REST confirmations and WebSocket events. Since Bybit REST responses often arrive before the corresponding WebSocket execution report, we cannot rely solely on one source.

### The Two States
1.  **Internal State (Optimistic):**
    *   Tracks what we *just* did or are about to do.
    *   Updated immediately when we send a REST request (e.g., "Order Sent", "Order Amending").
    *   Prevents double-submission while waiting for the network.
2.  **Exchange State (Confirmed):**
    *   The "Hard Truth" from Bybit.
    *   Updated by **BOTH** REST responses (immediate confirmation of OrderID) and WebSocket messages (async execution reports).

### Reconciliation Strategy (Hybrid REST/WS)
*   **On Action:** When `submit_order` returns via REST, we immediately update the *Exchange State* with the `order_id` and status `New`. We don't wait for the WS "New" message.
*   **On WS Event:** When the WebSocket pushes an update (e.g., `PartiallyFilled`), it merges into the *Exchange State*.
*   **Version Check:** Each state update carries a timestamp/sequence. If a "stale" WebSocket message arrives (e.g., "New" status) *after* we already processed the REST response, it is ignored or merged intelligently.
*   **Self-Healing:** If we miss a REST response (timeout), the WebSocket eventually arrives to fill the gap. If WS disconnects, the next REST action syncs the state.

## 2. The Execution Loop (Reconciliation)
This is the core logic ("check we cancel... or modify... or what is best"). We implement a **diff-based reconciliation loop**:

Every time the Gap Predictor generates new quotes (`TargetBid`, `TargetAsk`), the OMS runs this logic:

1.  **Check Existing:** Do I have an open order on this side?
2.  **Compare:**
    *   **If No Order:** $\rightarrow$ `Submit New Limit Order` (PostOnly).
    *   **If Order Exists & Price Matches:** $\rightarrow$ Do nothing (Best outcome).
    *   **If Order Exists & Price Changed:** $\rightarrow$ `Amend Order` (Modify).
        *   *Note:* modifying is preferred over Cancel+Replace because it's a single API call (faster) and sometimes preserves queue priority (depending on exchange matching engine rules, though usually price change loses priority).
3.  **Check Exposure:** Before sending any order, check `Inventory State`. If `CurrentPosition + NewOrder > MaxLimit`, clamp the size or skew quotes to reduce inventory.

## 3. Authentication & Connectivity
*   **Signing:** Bybit V5 requires HMAC-SHA256 signing. We implement a `Signer` struct that takes `api_key` and `secret`, generates the timestamp and signature, and attaches headers (`X-BAPI-SIGN`, etc.).
*   **Client:** We use `reqwest` (async HTTP client) for the actions (Submit/Cancel/Amend).
*   **Latency Optimization:** Re-use the same HTTPS connection (Keep-Alive) to avoid SSL handshake overhead on every order.

## Summary of Data Flow

```
[Market Data WS] --> [Gap Predictor (Logic)] --> [Target Quotes]
                                                        |
                                                        v
[Private WS] --> [OMS State (Position/Orders)] <--> [Reconciler]
      ^                         ^                       |
      |                         | (Updates)             | (Actions)
      |                         v                       v
      +----------------- [Bybit API (REST)] <-----------+
```

## Why this way?
*   **Accuracy:** We use the fastest available confirmation (REST or WS) to update state.
*   **Safety:** Dual-state prevents "phantom orders" where the bot thinks it has no orders because the WS is lagging, leading to double-buying.
*   **Speed:** Amending orders is faster than cancelling and replacing.
*   **Reliability:** The state is self-healing. If a REST call fails, the next WebSocket update brings the state back to truth.
