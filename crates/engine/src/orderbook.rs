use domain::{Order, Price};
use std::collections::{BTreeMap, VecDeque};

// An OrderBook tracks two separate lists: People buying, and people selling.
pub struct OrderBook {
    // BTreeMap<Key, Value>
    // Key = The Price
    // Value = A line (VecDeque) of Orders sitting at that exact price!
    pub bids: BTreeMap<Price, VecDeque<Order>>, // Buy orders
    pub asks: BTreeMap<Price, VecDeque<Order>>, // Sell orders
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) {
        // Depending on if the person is Buying or Selling, we pick the correct map!
        let map = match order.side {
            domain::Side::Buy => &mut self.bids,
            domain::Side::Sell => &mut self.asks,
        };

        // We find the line of people at that specific price.
        // If no one is waiting at that price yet, we create an empty line (VecDeque::new)
        let queue = map.entry(order.price).or_insert_with(VecDeque::new);

        // We put the new person at the back of the line!
        queue.push_back(order);
    }
    pub fn cancel_order(&mut self, order: Order) {
        let map = match order.side {
            domain::Side::Buy => &mut self.bids,
            domain::Side::Sell => &mut self.asks,
        };
        if let Some(line_order) = map.get_mut(&order.price) {
            line_order.retain(|o| o.id != order.id);
            if line_order.is_empty() {
                //does it cehck one by one
                map.remove(&order.price);
            }
        }
    }

    // pub fn match_order(&mut self, incoming_sell_order: Order) {
    //     let mut incoming_sell_order = incoming_sell_order;
    //     let bids = &mut self.bids;
    //     while let Some((price, line_of_orders)) = bids.iter_mut().last() {
    //         if *price >= incoming_sell_order.price {
    //             if let Some(best_bid) = line_of_orders.pop_front() {
    //                 &self.bids;
    //             }
    //         }
    //     }
    // }
    pub fn best_ask_price(&mut self) -> Option<&Price> {
        self.asks.first_key_value().map(|(price, _)| price)
    }

    pub fn best_bid_price(&self) -> Option<&Price> {
        self.bids.last_key_value().map(|(price, _)| price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{OrderId, OrderStatus, OrderType, Qty, Side, Symbol, UserId};

    // Helper function so we don't have to type this 100 times!
    fn create_buy_order(id: i64, price: i64) -> Order {
        Order {
            id: OrderId(id),
            user_id: UserId(99),
            symbol: Symbol(*b"BTC-USDT"),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Price(price),
            qty: Qty(10),
            filled_qty: Qty(0),
            timestamp: 12345,
            status: OrderStatus::Pending,
        }
    }

    fn create_sell_order(id: i64, price: i64) -> Order {
        Order {
            id: OrderId(id),
            user_id: UserId(99),
            symbol: Symbol(*b"BTC-USDT"),
            side: Side::Sell,
            order_type: OrderType::Limit,
            price: Price(price),
            qty: Qty(10),
            filled_qty: Qty(0),
            timestamp: 12345,
            status: OrderStatus::Pending,
        }
    }

    #[test]
    fn test_orderbook_sorting() {
        let mut book = OrderBook::new();

        // We add orders OUT OF ORDER maliciously!
        book.add_order(create_buy_order(1, 50_000)); // Alice buys at 50k
        book.add_order(create_buy_order(2, 49_000)); // Charlie buys at 49k
        book.add_order(create_buy_order(3, 51_000)); // Bob buys at 51k

        // Because these are BUY orders, the HIGHEST price should be processed first.
        // In a BTreeMap, the highest number is always at the back of the tree (.last_key_value)
        let (best_price, line_of_orders) = book.bids.last_key_value().unwrap();

        let first_person_in_line = line_of_orders.front().unwrap();

        // Prove Bob's order is first, even though he was added last!
        assert_eq!(*best_price, Price(51_000));
        assert_eq!(first_person_in_line.id, OrderId(3));

        println!("Test Passed: BTreeMap perfectly sorted Bob to the front of the line!");
    }

    #[test]
    fn test_best_bid_ask() {
        let mut book = OrderBook::new();

        book.add_order(create_buy_order(1, 50_000));
        book.add_order(create_buy_order(2, 51_000));
        // Add a few sell orders (asks)
        book.add_order(create_sell_order(3, 52_000));
        book.add_order(create_sell_order(4, 53_000));

        assert_eq!(book.best_bid_price(), Some(&Price(51000)));
        assert_eq!(book.best_ask_price(), Some(&Price(52000)));
    }

    #[test]
    fn test_cancel_order() {
        let mut book = OrderBook::new();
        book.add_order(create_buy_order(1, 50_000));
        book.add_order(create_buy_order(2, 51_000));
        book.add_order(create_buy_order(3, 51_000));

        book.cancel_order(create_buy_order(2, 51_000));

        assert_eq!(book.bids.get(&Price(51_000)).unwrap().len(), 1); 
    }
}
