# 🌐 `api` — REST API Server (The Front Door)

**One job: Accept HTTP requests from clients, validate them, and route them to the right internal system.**


## Test

#### Place a sell order (to create a matching price level):
```bash
curl -X POST http://localhost:8080/orders \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "user_id": 101,
    "symbol": [66, 67, 84, 45, 85, 83, 68, 84],
    "side": "sell",
    "order_type": "limit",
    "price": 50000,
    "qty": 10,
    "filled_qty": 0,
    "timestamp": 12345,
    "status": "pending"
  }'

#### Place a matching buy order:
curl -X POST http://localhost:8080/orders \
  -H "Content-Type: application/json" \
  -d '{
    "id": 2,
    "user_id": 102,
    "symbol": [66, 67, 84, 45, 85, 83, 68, 84],
    "side": "buy",
    "order_type": "limit",
    "price": 50000,
    "qty": 10,
    "filled_qty": 0,
    "timestamp": 12345,
    "status": "pending"
  }'

---

## 🤔 Why Does This Crate Exist?

The matching engine doesn't speak HTTP. The database doesn't speak HTTP. The `api` crate is the **translator** — it listens on port 8080, speaks HTTP to the outside world, and speaks Rust channels + SQL to the inside world.

> **Analogy:** Think of `api` as the front desk of a bank. A customer walks in and says "I want to buy 10 BTC!" (HTTP POST). The receptionist checks your ID (validates the order), checks your account balance (lock_funds), stamps a form (creates DB record), and puts the form on a conveyor belt to the back office (sends to engine via mpsc channel).

---

## 📦 What's Inside

### `main.rs` (The Startup Script)

When the server starts, it:

1. **Reads `.env`** — Gets the `DATABASE_URL` from the environment
2. **Creates DB Pool** — Opens a pool of connections to Postgres
3. **Creates Belt 1** — `mpsc::channel::<Order>(10000)` — the order intake pipeline
4. **Creates Belt 2** — `mpsc::channel::<Trade>(10000)` — the trade settlement pipeline
5. **Spawns the Matching Engine Task** — A `tokio::spawn` that runs the OrderBook in its own async task forever
6. **Spawns the Cashier Task** — Another `tokio::spawn` that listens for confirmed trades and settles them in Postgres
7. **Starts Actix-web Server** — Binds to `127.0.0.1:8080` and registers all route handlers

---

## 🛣️ Routes

| Method | Path | What It Does |
|---|---|---|
| `GET` | `/health_checker` | Returns "Hello All good!" — used to verify the server is running |
| `GET` | `/orders` | Returns all orders from the database as JSON |
| `POST` | `/orders` | Creates a new order (with risk check) |
| `DELETE` | `/order/:id` | Deletes an order by ID |

---

## 🔒 The Risk Check (POST /orders flow)

This is the most important flow in the API:

```
1. Client sends: POST /orders  { buy 10 BTC @ $65,000 }
        │
        ▼
2. Calculate cost: 10 × $65,000 = $650,000

3. lock_funds(user_id, "USD", $650,000)
        │
        ├── FAIL → 400 Bad Request: "Insufficient funds!"
        │
        └── SUCCESS:
              │
              ▼
        4. create_order(pool, order)  →  Saved to Postgres
              │
              ▼
        5. sender.send(order)  →  Order goes onto Belt 1 to the engine
              │
              ▼
        6. 200 OK: "Order created and funds locked!"
```

---

## ⚡ The Two Background Workers (tokio::spawn)

### Matching Engine Worker
```rust
tokio::spawn(async move {
    let mut orderbook = OrderBook::new(trade_tx);
    while let Some(order) = rx.recv().await {
        orderbook.match_order(order); // This is where trades happen!
    }
});
```
This lives in its own async task. While millions of users are hitting the API, this engine is quietly processing one order at a time in its own world.

### Cashier (Settlement) Worker  
```rust
tokio::spawn(async move {
    while let Some(trade) = trade_rx.recv().await {
        settle_trade(&pool, ...).await; // ACID transaction in Postgres
    }
});
```
Every trade confirmed by the engine flows here to update real balances in the database.

---

## 🔑 Key Rust Concepts

### `web::Data<T>` — Shared State Between Threads
Each incoming HTTP request is handled by a different Actix worker thread. To share the `PgPool` and `mpsc::Sender` safely across all of them, we wrap them in `web::Data<T>` (which is essentially `Arc<T>` under the hood).

```rust
.app_data(web::Data::new(db_pool))    // All handlers can access the DB pool
.app_data(web::Data::new(tx))         // All handlers can send orders to the engine
```

### `tokio::spawn` — Fire-and-Forget Async Tasks
The engine and cashier are started with `tokio::spawn`. This creates an independent async green thread that runs concurrently with the web server without blocking it.

---

## 🏃 Running

```bash
# Start Postgres first!
docker-compose up -d

# Start the API
cargo run -p api

# Test it
curl http://localhost:8080/health_checker
curl http://localhost:8080/orders
```

---

## 🔗 What This Crate Imports

```
domain  ← for Order, Trade, Side, etc.
db      ← for create_order, lock_funds, settle_trade, get_orders, delete_order
engine  ← for OrderBook (spawned inside the matching engine task)
```
