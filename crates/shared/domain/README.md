# 📐 `domain` — Shared Types & Contracts

**One job: Define what every piece of data looks like across the whole exchange.**

---

## 🤔 Why Does This Crate Exist?

Without this crate, the `api` crate, the `engine` crate, and the `db` crate would all have their own version of `Order`. They would get out of sync. The compiler wouldn't know if they were the same thing.

`domain` is the **single language everyone speaks**. If you add a field to `Order` here, the compiler will instantly point to every crate that needs to be updated.

> **Analogy:** Think of `domain` as the official rulebook of a board game. Before anyone plays (api, engine, db), they all read the SAME rulebook. If the rulebook says a Bishop moves diagonally, EVERYONE agrees — there is no debate.

---

## 📦 What's Inside

### Newtypes (Wrappers around `i64`)

```rust
pub struct OrderId(pub i64);
pub struct UserId(pub i64);
pub struct Price(pub i64);   // Stored as integer cents: $50,000 = 5000000
pub struct Qty(pub i64);
```

**Why Newtypes?**  
Prevents accidental mixups. Without newtypes, `fn fill(order_id: i64, user_id: i64)` could be called as `fill(user_id, order_id)` and the compiler wouldn't notice. With newtypes, calling `fill(UserId(1), OrderId(99))` causes a **compile-time error**. The wrong call is impossible.

---

### Enums

```rust
pub enum Side { Buy, Sell }
pub enum OrderType { Market, Limit, StopLimit }
pub enum OrderStatus { Pending, PartialFilled, Filled, Cancelled }
```

These map directly to **Postgres custom ENUM types** via `#[sqlx(type_name = "side")]`.

---

### Core Structs

**`Order`** — A single instruction from a user: "I want to buy 10 BTC at $65,000."
```
id, user_id, symbol, side, order_type, price, qty, filled_qty, timestamp, status
```

**`Trade`** — A completed match between a buyer and a seller. This is emitted by the matching engine every time two orders cross.
```
maker_user_id, taker_user_id, symbol, price, qty
```

**`Candle`** — An aggregated summary of all trades in a time window (1 min, 5 min, etc).
```
symbol, interval_start, open, high, low, close, volume, total_quote_qty
```
The `vwap()` method calculates the **Volume Weighted Average Price**: total money exchanged ÷ total quantity. The most accurate measure of "true price" in a time window.

---

## ❓ Why Are Prices `i64`, Not `f64`?

`f64` floating point cannot represent `0.1 + 0.2` exactly. Try it in Python:
```python
>>> 0.1 + 0.2
0.30000000000000004
```

In a financial exchange, this rounding error would cause money to vanish or appear out of nothing. We store prices as integer cents:
- `$65,000.00` is stored as `6500000`
- Math on integers is always exact

---

## 🔗 Who Imports This?

```
domain  ← db
domain  ← engine
domain  ← api
domain  ← market_data
```

**Everyone imports `domain`. `domain` imports nothing from the workspace.** This is intentional — `domain` must never depend on implementation details.
