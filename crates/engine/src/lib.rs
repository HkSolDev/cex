use domain::*;
use tokio::sync::mpsc;
use std::time::Duration;

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
}
impl MatchingEngine {
    //Did not understand what this will do
    pub fn new(buffer_size: usize) -> (Self, OrderSender) {
        let (tx, rx) = mpsc::channel(buffer_size);
        (MatchingEngine { rx }, OrderSender { tx })
    }

    pub async fn run(mut self) {
        println!("Matching Engine started... waiting for orders in the pipeline.");

        while let Some(order) = self.rx.recv().await {
            println!("Received order: {:?}", order);
            // Process the order here
        }
    }
}


#[cfg(test)]
mod tests {
use domain::{Qty, Side};

use super::*;

#[tokio::test]
pub async fn test_order_intake_pipeline(){
    let (mut engine, sender) = MatchingEngine::new(10);

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
    sender.send(order.clone()).await.expect("Failed to send order");
    let receive_order =tokio::time::timeout(Duration::from_millis(100), engine.rx.recv()).await.expect("Engine timed out waiting for order! Pip is broken!").expect("Channel was closed unexpectedly");
    assert_eq!(receive_order.id, order.id);
    println!("Test Passed: Order successfully traveled through the pipeline to the Engine!");
}
}

