pub mod orderbook;
use domain::*;

use tokio::sync::{broadcast, mpsc};

#[derive(Clone)]
pub struct OrderSender {
    tx: mpsc::Sender<Order>,
}
impl OrderSender {
    pub async fn send(&self, order: Order) -> Result<(), mpsc::error::SendError<Order>> {
        self.tx.send(order).await?;
        Ok(())
    }
}

pub struct MatchingEngine {
    rx: mpsc::Receiver<Order>,
    trade_tx: broadcast::Sender<Trade>,
}
impl MatchingEngine {
    //Did not understand what this will do
    pub fn new(buffer_size: usize) -> (Self, OrderSender, broadcast::Sender<Trade>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        //To broadcast the trade to db websocket
        let (trade_tx, _) = broadcast::channel(1000);
        // Process the order here

        (
            MatchingEngine {
                rx,
                trade_tx: trade_tx.clone(),
            },
            OrderSender { tx },
            trade_tx,
        )
    }

    pub async fn run(mut self) {
        println!("Matching Engine started... waiting for orders in the pipeline.");

        while let Some(order) = self.rx.recv().await {
            println!("Received order: {:?}", order);
            let dummy_trade = Trade {
                maker_user_id: 101, // Some random guy waiting in the orderbook
                taker_user_id: 0,   // The person who just sent the aggressive order
                symbol: *b"BTC-USDT",
                price: 65000_00, // 65,000 USD
                qty: 5,
                timestamp: 1234567890,
            };

            // Now shout it out of the megaphone!
            // We use .ok() to silently ignore the error if no one is listening right now.
            self.trade_tx.send(dummy_trade).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use domain::{Qty, Side};
    use std::time::Duration;

    use super::*;


    #[tokio::test]
    pub async fn test_order_intake_pipeline() {
        let (mut engine, sender, _trade_tx) = MatchingEngine::new(10);

        let order = Order {
            id: OrderId(1),
            user_id: UserId(99),
            symbol: Symbol(*b"BTC-USDT"), // *b turns string into [u8; 8]
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Price(50000_00),
            qty: Qty(10),
            filled_qty: Qty(0),
            timestamp: 12345,
            status: OrderStatus::Pending,
        };
        sender
            .send(order.clone())
            .await
            .expect("Failed to send order");
        let receive_order = tokio::time::timeout(Duration::from_millis(100), engine.rx.recv())
            .await
            .expect("Engine timed out waiting for order! Pip is broken!")
            .expect("Channel was closed unexpectedly");
        assert_eq!(receive_order.id, order.id);
        println!("Test Passed: Order successfully traveled through the pipeline to the Engine!");
    }
}
