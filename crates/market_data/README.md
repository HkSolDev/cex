# 📊 `market_data` — Candlestick & OHLCV Engine

**One job: Listen to every trade that happens on the exchange and aggregate it into beautiful candlestick (OHLCV) data in real-time.**

---

## 🤔 Why Does This Crate Exist?

The Matching Engine produces raw trades: "User 101 just sold 5 BTC to User 99 at $65,000." That's useful for settlement, but useless for a user trying to understand the market.

Users need to see: "In the last minute, BTC started at $64,900, went up to $65,100, dipped to $64,850, and closed the minute at $65,000 on 42 BTC of volume." That is a **candlestick**.

This crate is an accountant constantly aggregating raw trades into meaningful patterns in memory.

> **Analogy:** The Matching Engine is a cashier at a supermarket, scanning items 1,000 times a second and shouting the price out loud. The `market_data` engine is the store manager sitting in the back room, listening to the radio, writing down on a notebook: "This minute's total was $X, the most expensive item was $Y, the cheapest was $Z." At the end of every minute, they take a photo of the notebook page — that photo is the **Candle**.

---

## 📊 What is OHLCV?

Every candle represents all the trades in a fixed time window (1 minute, 5 minutes, 1 hour):

```
Price
  │      ┌──── High ($65,100) ────┐
  │      │                        │
$65,100 ─┼─         ▲ wick        │
  │      │           │            │
$65,000 ─┼── Open ───┤      Close ┤── Close ($65,000)
  │      │           │            │
$64,900 ─┼──         │            │
  │      │           ▼ wick       │
$64,850 ─┼─         ▼ Low ────────┘
  │
  └─────────────────────────────────── Time
       Start of minute   End of minute
```

| Field | Meaning |
|---|---|
| **O**pen | Price of the very FIRST trade in the minute |
| **H**igh | Highest price any trade happened at in the minute |
| **L**ow | Lowest price any trade happened at in the minute |
| **C**lose | Price of the very LAST trade in the minute |
| **V**olume | Total quantity of the asset that was traded |

---

## 📐 VWAP — Volume Weighted Average Price

VWAP (pronounced "vee-wap") is the "true average price" of the minute, weighted by how much was traded at each price.

```
Simple Average:    (65,000 + 65,050 + 64,900) / 3 = $64,983  ← treats all trades equally
VWAP:              (65,000×100 + 65,050×5 + 64,900×500) / 605 ← weights by volume
```

A trade of 500 BTC at $64,900 should count WAY more than a test trade of 5 BTC at $65,050. VWAP captures this correctly.

```rust
pub fn vwap(&self) -> i64 {
    self.total_quote_qty / self.volume  // Total $ traded ÷ Total BTC traded
}
```

---

## 🏗️ Design: Time Bucketing

We store candles in a `HashMap<([u8; 8], i64), Candle>`:
- Key Part 1: `[u8; 8]` — the market symbol (e.g. `BTC-USDT`)
- Key Part 2: `i64` — the Unix timestamp rounded DOWN to the nearest minute

```rust
// How to round a Unix timestamp DOWN to the nearest minute:
let minute_start = (trade.timestamp / 60) * 60;
//                  ↑ integer division drops the seconds
//                                   ↑ multiply back to get the minute boundary
```

Example:
- Trade at `16:32:47` → minute bucket = `16:32:00`
- Trade at `16:32:59` → minute bucket = `16:32:00` (same bucket!)
- Trade at `16:33:01` → minute bucket = `16:33:00` (new bucket!)

---

## 📦 The `Candle` Struct (in `domain`)

```rust
pub struct Candle {
    pub symbol: [u8; 8],
    pub interval_start: i64, // Start of the minute (Unix timestamp)
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub volume: i64,
    pub total_quote_qty: i64,  // Sum of (price * qty) — used for VWAP
}
```

Two key methods:
- `Candle::new(&trade)` — Creates a brand new candle from the very first trade in a minute
- `candle.update(&trade)` — Updates an existing candle when another trade arrives in the same minute

---

## 🔗 How It Connects

```
engine (broadcast::Sender<Trade>)
       │
       ▼
market_data subscribes via trade_tx.subscribe()
       │
       ▼
CandleEngine::run() — loops forever
       │
       ├── On each trade: find or create the Candle bucket
       │   candles.entry((symbol, minute)).or_insert(Candle::new(&trade))
       │                                   .update(&trade)
       │
       └── (Future) Publish updated Candle to WebSocket clients
```

---

## 🏢 How Binance Does This (vs Us)

| What | Us | Binance |
|---|---|---|
| Storage | `HashMap` in RAM | TimescaleDB (Postgres extension for time-series) |
| Intervals | 1 minute (MVP) | 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M |
| Delivery | (Coming) WebSocket | WebSocket stream `btcusdt@kline_1m` |
| Throughput | In-process | Distributed Kafka consumers |

Our in-memory `HashMap` approach is actually identical to what Binance does for the **live streaming candle** — only the historical persistence uses a time-series database.

---

## ⚙️ Status

🚧 **Under Active Construction** — This crate was just scaffolded.  
Next step: Implementing `CandleEngine` with `HashMap`-based time bucketing.
