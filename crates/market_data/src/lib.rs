use domain::{Candle, Trade};
use std::collections::{BTreeMap, HashMap};
use tokio::sync::broadcast;

pub struct CandleEngine {
    /// Key: (Symbol Bytes, Minute Unix Timestamp)
    pub candles: BTreeMap<([u8; 8], i64), Candle>,
}

impl CandleEngine {
    pub fn new() -> Self {
        Self {
            candles: BTreeMap::new(),
        }
    }

    pub async fn run(&mut self, mut trade_rx: broadcast::Receiver<Trade>) {
        println!("Candle Engine started... waiting for trades.");

        loop {
            match trade_rx.recv().await {
                Ok(trade) => {
                    // Time Bucket Formula: Round down timestamp to the nearest 60 seconds ex 10:05:23 & 10:05:59 both come at 10:05:00 (1 minute)
                    let minute_start = (trade.timestamp / 60) * 60;

                    // Fetch or Create the candle for this (symbol, minute)
                    let candle = self
                        .candles
                        .entry((trade.symbol, minute_start))
                        .or_insert(Candle::new(&trade, minute_start));

                    // Update it with the latest price/qty what it mean the update at the end of the minute  sovel the doubt
                    candle.update(&trade);

                    println!("Updated Candle: {:?}", candle);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("Candle Engine lagged behind by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    println!("Trade broadcast channel closed. Candle Engine shutting down.");
                    break;
                }
            }
        }
    }
}
