# 🏦 CEX — High-Performance Rust Centralized Exchange

A **Binance-style centralized exchange (CEX) backend** built in Rust from scratch.  
This is a learning-by-building project: every Rust concept is introduced because a real exchange component needs it.

---

## 🏗️ Architecture Overview

This is a **Cargo Workspace** — a single monorepo containing multiple independent crates (microservices), each owning exactly one job.

```
Client (HTTP / WebSocket)
         │
         ▼
┌─────────────────────┐
│   crates/api        │  ← REST API (Actix-web)
│   POST /orders      │    Receives orders from clients
│   GET  /orders      │    Validates funds (Risk Check)
│   DELETE /order/:id │    Sends orders to the Engine belt
└────────┬────────────┘
         │  mpsc::Sender<Order>   (Belt 1: Order Pipeline)
         ▼
┌─────────────────────┐
│   crates/engine     │  ← Matching Engine + OrderBook
│   OrderBook         │    BTreeMap-sorted bids/asks
│   MatchingEngine    │    Matches Buy vs Sell (FIFO)
│   Partial Fills     │    Loops until order is filled
└──────┬──────────────┘
       │  mpsc::Sender<Trade>    (Belt 2: Trade Pipeline)
       │  broadcast::Sender<Trade> (Megaphone: Real-time feeds)
       ├─────────────────────────────────┐
       ▼                                 ▼
┌──────────────────┐         ┌──────────────────────┐
│  crates/shared   │         │  crates/market_data  │
│  /db  (Cashier)  │         │  CandleEngine (OHLCV)│
│  settle_trade()  │         │  VWAP calculator     │
│  ACID Postgres   │         │  in-memory HashMap   │
└──────────────────┘         └──────────────────────┘
       │
       ▼
┌──────────────────┐
│    PostgreSQL    │  ← Permanent storage (balances, trades, orders)
└──────────────────┘
```

---

## 📦 Crate Structure

| Crate | Path | Role |
|---|---|---|
| `domain` | `crates/shared/domain` | All shared types: `Order`, `Trade`, `Candle`, `Side`, etc. |
| `db` | `crates/shared/db` | All database functions: `create_order`, `settle_trade`, `lock_funds` |
| `engine` | `crates/engine` | OrderBook (`BTreeMap`) + Matching loop + broadcast channel |
| `api` | `crates/api` | Actix-web REST server. Entry point of the system |
| `market_data` | `crates/market_data` | Candle (OHLCV) aggregation engine, VWAP |

---

## 🔗 How Crates Connect

```
domain  ←──── db
domain  ←──── engine
domain  ←──── api
db      ←──── api          (api calls db to persist data)
engine  ←──── api          (api spawns the matching engine)
engine  ←──── market_data  (market_data subscribes to engine's broadcast)
```

`domain` is the **source of truth** for all types. Every other crate imports from it. 
This means if you change the `Order` struct, the compiler will instantly tell every crate that is affected.

---

## ⚙️ The Two Belts (Channel Architecture)

Think of the system as a factory with conveyor belts:

**Belt 1 — Order Pipeline** (`tokio::sync::mpsc`)
```
API Handler  ──► [mpsc Belt]  ──► MatchingEngine loop
```
- One-to-One: Only the engine listens on this belt
- Used because: orders must be processed ONE AT A TIME (deterministic matching)

**Belt 2 — Trade Pipeline** (`tokio::sync::mpsc`)
```
MatchingEngine ──► [mpsc Belt]  ──► Cashier (DB Writer)
```
- One-to-One: Only the Cashier (DB settlement) listens
- Used because: settlement is a serial, ordered operation (ACID)

**Belt 3 — Broadcast Megaphone** (`tokio::sync::broadcast`)
```
MatchingEngine ──► [broadcast channel]  ──► CandleEngine
                                        ──► WebSocket feeds
                                        ──► Volume Analytics
```
- One-to-Many: Many services subscribe to the same traded event
- Used because: multiple independent services need the same data

---

## 🗄️ Database Design

PostgreSQL with `sqlx` compile-time verified queries.

Tables: `users`, `orders`, `balances`

Key design decisions:
- **Prices & Quantities stored as `i64` (integer cents)** — Never use `f64` for money. `0.1 + 0.2 ≠ 0.3` in floating point!
- **`sqlx::query!` macro** — SQL is verified against the live DB at compile time. If SQL is wrong, the code won't compile.
- **ACID Transactions** — `settle_trade` wraps all balance updates in a single transaction. Either all 4 balance updates succeed, or none of them do. No half-trades.

---

## 🚀 Running Locally

**Prerequisites:** Docker, Rust (stable)

```bash
# 1. Start the database
docker-compose up -d

# 2. Run migrations
sqlx migrate run

# 3. Start the API server
cargo run -p api

# 4. Test an endpoint
curl -X GET http://localhost:8080/health_checker
```

---

## 📚 Rust Concepts Learned (In Order)

| Phase | What We Built | Rust Concepts |
|---|---|---|
| 0 | Workspace scaffold | `cargo workspace`, crate structure, modules |
| 1 | Order types + Channel pipeline | `struct`, `enum`, `impl`, `mpsc::channel` |
| 2 | Order Book | `BTreeMap`, `VecDeque`, sorted collections |
| 3 | Matching Engine | `loop`, partial fills, mutable refs |
| 4 | Accounts + Settlement | `HashMap`, `Result`, ACID DB transactions |
| 5 | Market Data + Broadcasts | `broadcast::channel`, time bucketing, OHLCV |
| 6 | REST + WebSocket (Coming) | Actix-web actors, async streams |

---

## 🏢 How Binance Does This (vs Us)

| Component | Our Implementation | Binance Production |
|---|---|---|
| Order Pipeline | `tokio::mpsc` in-process | Apache Kafka across servers |
| Order Book | `BTreeMap` in RAM | Custom C++ red-black tree |
| Real-time Feeds | `tokio::broadcast` in-process | Redis Pub/Sub |
| Market Data Storage | In-memory `HashMap` | TimescaleDB (Postgres extension) |
| Settlement | `sqlx` ACID transaction | Distributed saga pattern |

---

## ⚠️ Known Gotchas

1. `cargo check` requires the **DB to be running** because `sqlx::query!` connects at compile time. Always run `docker-compose up -d` first.
2. Never edit an applied migration file — Postgres caches checksums. Always `sqlx migrate add new_migration_name`.
3. `broadcast::Receiver` will return `RecvError::Lagged` if it falls behind. Handle this error or the slow subscriber will miss messages silently.
