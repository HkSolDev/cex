use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OrderId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Symbol(pub [u8; 8]); // E.g., "BTC-USD".

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Price(pub u64); // Integer cents

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Qty(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] 
//Why we use the partialEq and clone is cloen we use as it share all across the project 
pub enum OrderStatus {
    Pending,
    PartialFille,
    Fill,
    Cancelled
}

#[derive(Debug, Clone, PartialEq,Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub user_id: UserId,
    pub symbol: Symbol,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Price,
    pub qty: Qty,
    pub filled_qty: Qty,
    pub timestamp: u64,
    pub status: OrderStatus,
}

impl Order {
    pub fn filled(&self) -> bool {
        self.qty == self.filled_qty
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppError {
    Internal(String),
    Validation(String),
    NotFound(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newtype_equality() {
        let order1 = OrderId(1);
        let order2 = OrderId(1);
        let order3 = OrderId(2);

        assert_eq!(order1, order2);
        assert_ne!(order1, order3);
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError::Validation("Invalid price".to_string());
        assert_eq!(format!("{}", err), "Validation error: Invalid price");
    }
}
