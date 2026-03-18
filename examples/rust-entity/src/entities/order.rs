use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub title: String,
    pub amount: Option<rust_decimal::Decimal>,
    pub shipped: bool,
}

impl Order {
    // <ai:customize hint="Add constructor and domain methods for Order">
    pub fn new(id: String) -> Self {
        todo!()
    }
    // </ai:customize>
}
