# 🗄️ `db` — Database Layer (The Cashier)

**One job: Talk to PostgreSQL. No one else in the system is allowed to.**

---

## 🤔 Why Does This Crate Exist?

The matching engine should not write SQL. The API should not write SQL. If they did, the SQL would be scattered everywhere in 10 different files and completely unmaintainable.

`db` is the **only crate that is allowed to touch Postgres**. Every other crate that needs data stored calls a function from this crate.

> **Analogy:** Think of `db` as the bank vault. Your `api` is a customer — it can ask the bank teller to deposit or withdraw. But the customer is NEVER allowed to open the vault themselves. The `db` crate is the teller + vault combo.

---

## 📦 What's Inside

### `create_connection_pool(url) → PgPool`
Creates a pool of database connections at startup. A pool is a set of pre-opened connections sitting on standby. When a request comes in, it grabs one from the pool instead of paying the cost of creating a new one from scratch each time.

---

### `create_order(pool, order)` 
Writes a new `Order` to the `orders` table. Called by the API after the risk check passes.

---

### `get_orders(pool) → Vec<Order>`
Reads all orders from the database. Used by the `GET /orders` API endpoint.

---

### `delete_order(pool, id)`
Removes an order by ID. Used by the `DELETE /order/:id` endpoint.

---

### `lock_funds(pool, user_id, asset, amount)` — The Risk Bouncer
**This is the most important safety function in the whole system.**

Before any order is accepted, we check if the user actually has the money. If a Buy order comes in for `10 BTC × $65,000 = $650,000`, we immediately move `$650,000` from their `free` balance to their `locked` balance.

```
free balance:   $700,000  →  $50,000    (decreased)
locked balance: $0        →  $650,000   (increased)
```

This prevents a user from placing 10 simultaneous orders with the same $650,000 (double-spend attack).

If the user doesn't have enough in `free`, returns `DomainError::InsufficientFunds` and the order is rejected.

---

### `settle_trade(pool, maker_id, taker_id, base, quote, base_qty, quote_qty)`
**The ACID nuclear weapon.**

This runs inside a single Postgres **transaction**, meaning all 4 balance updates succeed together, or ALL of them are rolled back. No partial states. No money created from nothing.

The 4 operations in one atomic transaction:
1. `maker.locked_BTC -= trade_qty` (maker sold their locked BTC)
2. `maker.free_USD += total_usd` (maker receives USD payment)
3. `taker.locked_USD -= total_usd` (taker's locked cash is released)
4. `taker.free_BTC += trade_qty` (taker receives their BTC)

---

## 🔑 Key Rust & Design Decisions

### `sqlx::query!` macro — Compile-Time SQL Verification
```rust
sqlx::query!("SELECT * FROM orders WHERE id = $1", id)
```
This is NOT a runtime string. At `cargo check` time, `sqlx` connects to the **real database** and verifies that:
- The table `orders` exists
- The column types match the Rust types
- The number of `$1` params matches the arguments

If the SQL is wrong, **the code doesn't compile.** No runtime surprises.

> ⚠️ **Important:** This means `cargo check` requires the DB to be running! Always `docker-compose up -d` first.

### ACID Transactions (`&mut *tx`)
```rust
let mut tx = pool.begin().await?;
sqlx::query!(...).execute(&mut *tx).await?;
sqlx::query!(...).execute(&mut *tx).await?;
tx.commit().await?;
```
`&mut *tx` forces ALL queries to go through the SAME database connection holding the lock. This is what makes the atomicity guarantee work.

---

## 🔗 Who Imports This?

```
db ← api   (API calls lock_funds, create_order, get_orders, delete_order)
db ← api   (Cashier inside api/main.rs calls settle_trade)
```

`engine` and `market_data` do NOT import `db`. They work entirely in memory.
