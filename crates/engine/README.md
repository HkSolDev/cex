# ⚙️ `engine` — Matching Engine & Order Book

**One job: Match incoming buy and sell orders as fast as possible, entirely in RAM.**

---

## 🤔 Why Does This Crate Exist?

The core of every exchange is in here. This code runs on its own dedicated async task, completely isolated from the HTTP server. It receives orders through a channel, matches them, and shouts the results out.

> **Analogy:** This is the auctioneer at a live auction. People (API clients) shout bids and asks to the auctioneer (MatchingEngine). The auctioneer holds a sorted list (OrderBook), and the second a buyer's price meets a seller's price, they smack the gavel and declare a trade — without ever pausing the auction.

---

## 📦 What's Inside

### `OrderBook` (`orderbook.rs`)

The core data structure. Holds two `BTreeMap`s — one for buyers, one for sellers.

```
BIDS (Buyers)                    ASKS (Sellers)
─────────────────────            ─────────────────────
Price $51,000 │ [Bob]            Price $52,000 │ [Dave]
Price $50,000 │ [Alice, Charlie] Price $53,000 │ [Eve]
Price $49,000 │ [Frank]          Price $54,000 │ [Frank]
      ▲                                  ▲
 Best Bid = $51k               Best Ask = $52k
 (highest buyer)               (lowest seller)
```

**Why `BTreeMap`?** It keeps entries always sorted by price. `O(log n)` insert and lookup. Finding the best bid/ask is instant: `.last_key_value()` for bids and `.first_key_value()` for asks.

**Why `VecDeque` inside the `BTreeMap`?** Multiple orders can sit at the same price. `VecDeque` gives us FIFO ordering (First In First Out) — the first person to place an order at `$51,000` gets matched first. This is **Price-Time Priority** — the international standard.

#### Methods
- `add_order(order)` — Puts an order in the correct price level queue
- `cancel_order(order)` — Removes a specific order from its price level. Cleans up empty price levels automatically.
- `match_order(order)` — The main matching loop. Loops until the incoming order is fully filled or no counterparty exists.
- `best_bid_price()` / `best_ask_price()` — Returns the current market's top-of-book price.

---

### `MatchingEngine` (`lib.rs`)

The async coordinator that owns the `OrderBook` and runs the event loop.

```
                 mpsc::channel (Belt 1)
API ──────────► [order queue] ──────► MatchingEngine::run()
                                              │
                                              ▼
                                        OrderBook.match_order()
                                              │
                                    ┌─────────┴──────────┐
                                    ▼                     ▼
                           mpsc (Belt 2)         broadcast (Megaphone)
                           → Cashier (DB)       → CandleEngine, WebSockets
```

#### Why one dedicated async task?
Matching MUST be **single-threaded and sequential**. If two threads matched orders simultaneously, they could both see the same counterparty order and generate two conflicting trades. The `mpsc` channel guarantees that orders arrive one at a time in a queue — the engine processes them in strict order.

#### The Broadcast Channel (Megaphone)
When a trade happens, the engine retains a `broadcast::Sender<Trade>`. It shouts the trade to ALL subscribers simultaneously.

- If a subscriber falls behind, they receive `RecvError::Lagged(n)` and are told they missed `n` messages.
- The engine is NEVER slowed down waiting for a slow subscriber.

---

## 🔑 Key Rust Concepts

### `BTreeMap` — Always-Sorted Map
```rust
// Key insight: BTreeMap<Price, VecDeque<Order>>
// - Price is the key, so the map is ALWAYS sorted by price
// - .last_key_value() = highest buyer in O(1) time
// - .first_key_value() = lowest seller in O(1) time
```
[Docs: BTreeMap](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)

### `tokio::sync::mpsc` (Belt 1 & 2)
Multi-producer, single-consumer channel. Many API threads produce orders, one engine thread consumes them. Used for `Order` intake and `Trade` settlement pipelines.
[Docs: tokio mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)

### `tokio::sync::broadcast` (Megaphone)
Single-producer, multi-consumer. The engine shouts `Trade` events to everyone subscribed. Used for real-time feeds.
[Docs: tokio broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)

---

## 🔗 Who Imports This?

```
engine ← api  (API spawns MatchingEngine and holds the OrderSender)
```

`market_data` subscribes to the broadcast channel returned by `MatchingEngine::new()`.

---

## 🧪 Tests

Run with:
```bash
cargo test -p engine
```

Tests live in `orderbook.rs`:
- `test_orderbook_sorting` — Proves `BTreeMap` puts the highest buyer at the front regardless of insertion order.
- `test_best_bid_ask` — Verifies `best_bid_price()` and `best_ask_price()` return correctly.
- `test_cancel_order` — Ensures cancellation removes the correct order and cleans up empty price levels.
