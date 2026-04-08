use std::collections::HashMap;

use crate::{error::TradeError, trade::Trade};

mod trade;
mod error;

pub struct TradeRegistry {
    trades: HashMap<u64, trade::Trade>,
    next_id: u64
}

impl TradeRegistry {
    pub fn new() -> TradeRegistry {
        TradeRegistry { trades: HashMap::new(), next_id: 1 }
    }
    pub fn create_trade(&mut self, requester_id: u64, details: trade::TradeDetails) -> Result<u64, TradeError> {
        match details.validate() {
            Ok(_) => {
                let id = self.next_id;
                self.next_id += 1;
                self.trades.insert(id, Trade::new(id, requester_id, details));
                Ok(id)
            }
            Err(_) => Err(TradeError::NotValid)
        }
    }
    
    pub fn get_trade(&self, trade_id: u64) -> Result<&trade::Trade, ()> {
        match self.trades.get(&trade_id) {
            Some(trade) => Ok(trade),
            None => Err(()),
        }
    }
}
