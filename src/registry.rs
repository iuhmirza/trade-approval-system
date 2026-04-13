use std::{collections::HashMap, fmt};

use chrono::NaiveDate;

use crate::{
    error::TradeError,
    history::{History, HistoryId},
    trade::{Direction, NotionalCurrency, Trade, TradeDetails, TradeId, TradeVersion},
    user::UserId,
};

pub struct TradeRegistry {
    trades: HashMap<TradeId, Trade>,
    next_trade_id: u64,
}

impl TradeRegistry {
    pub fn new() -> TradeRegistry {
        TradeRegistry {
            trades: HashMap::new(),
            next_trade_id: 1,
        }
    }

    pub fn next_trade_id(&mut self) -> TradeId {
        let id = TradeId::new(self.next_trade_id);
        self.next_trade_id += 1;
        id
    }

    pub fn create_trade(
        &mut self,
        requester_id: UserId,
        mut details: TradeDetails,
    ) -> Result<TradeId, TradeError> {
        match details.validate() {
            Ok(_) => {
                let id = self.next_trade_id();
                details.version = TradeVersion::new(1);
                self.trades
                    .insert(id, Trade::new(id, requester_id, details));
                Ok(id)
            }
            Err(_) => Err(TradeError::NotValid),
        }
    }

    pub fn get_trade(&self, trade_id: TradeId) -> Result<&Trade, TradeError> {
        match self.trades.get(&trade_id) {
            Some(trade) => Ok(trade),
            None => Err(TradeError::NotFound),
        }
    }

    pub fn get_history(&self, trade_id: TradeId) -> Result<&Vec<History>, TradeError> {
        self.get_trade(trade_id).map(|trade| &trade.history)
    }

    pub fn get_history_at(
        &self,
        trade_id: TradeId,
        history_id: HistoryId,
    ) -> Result<&History, TradeError> {
        let history = self.get_history(trade_id)?;
        let history = history.iter().find(|h| h.id == history_id);
        match history {
            Some(history) => Ok(history),
            None => Err(TradeError::NotFound),
        }
    }

    pub fn diff_version(
        &self,
        trade_id: TradeId,
        version_one: TradeVersion,
        version_two: TradeVersion,
    ) -> Result<(&TradeDetails, &TradeDetails), TradeError> {
        let trade = self.get_trade(trade_id)?;
        let versions = trade.versions();
        let version_one = versions
            .iter()
            .find(|v| v.version == version_one)
            .ok_or(TradeError::NotFound)?;
        let version_two = versions
            .iter()
            .find(|v| v.version == version_two)
            .ok_or(TradeError::NotFound)?;

        Ok((version_one, version_two))
    }

    pub fn submit(
        &mut self,
        trade_id: TradeId,
        user_id: UserId,
        notes: String,
    ) -> Result<(), TradeError> {
        self.trades
            .get_mut(&trade_id)
            .ok_or(TradeError::NotFound)?
            .submit(user_id, notes)
    }

    pub fn accept(
        &mut self,
        trade_id: TradeId,
        user_id: UserId,
        notes: String,
        mut details: Option<TradeDetails>,
    ) -> Result<(), TradeError> {
        let trade = self.trades.get_mut(&trade_id).ok_or(TradeError::NotFound)?;
        if let Some(details) = &mut details {
            let version = trade.details.version.raw();
            details.version = TradeVersion::new(version + 1)
        }
        trade.accept(user_id, notes, details)
    }

    pub fn approve(
        &mut self,
        trade_id: TradeId,
        user_id: UserId,
        notes: String,
    ) -> Result<(), TradeError> {
        self.trades
            .get_mut(&trade_id)
            .ok_or(TradeError::NotFound)?
            .approve(user_id, notes)
    }
    pub fn cancel(
        &mut self,
        trade_id: TradeId,
        user_id: UserId,
        notes: String,
    ) -> Result<(), TradeError> {
        self.trades
            .get_mut(&trade_id)
            .ok_or(TradeError::NotFound)?
            .cancel(user_id, notes)
    }
    pub fn send_to_execute(
        &mut self,
        trade_id: TradeId,
        user_id: UserId,
        notes: String,
    ) -> Result<(), TradeError> {
        self.trades
            .get_mut(&trade_id)
            .ok_or(TradeError::NotFound)?
            .send_to_execute(user_id, notes)
    }
    pub fn book(
        &mut self,
        trade_id: TradeId,
        user_id: UserId,
        notes: String,
        strike: String,
    ) -> Result<(), TradeError> {
        self.trades
            .get_mut(&trade_id)
            .ok_or(TradeError::NotFound)?
            .book(user_id, notes, strike)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TradeDifference {
    pub trading_entity: Option<(String, String)>,
    pub counterparty: Option<(String, String)>,
    pub direction: Option<(Direction, Direction)>,
    pub style: Option<(String, String)>,
    pub notional_currency: Option<(NotionalCurrency, NotionalCurrency)>,
    pub notional_amount: Option<(u64, u64)>,
    pub underlying: Option<(String, String)>,
    pub trade_date: Option<(NaiveDate, NaiveDate)>,
    pub value_date: Option<(NaiveDate, NaiveDate)>,
    pub delivery_date: Option<(NaiveDate, NaiveDate)>,
    pub strike: Option<(Option<String>, Option<String>)>,
}

impl TradeDifference {
    pub fn is_empty(&self) -> bool {
        self.trading_entity.is_none()
            && self.counterparty.is_none()
            && self.direction.is_none()
            && self.style.is_none()
            && self.notional_currency.is_none()
            && self.notional_amount.is_none()
            && self.underlying.is_none()
            && self.trade_date.is_none()
            && self.value_date.is_none()
            && self.delivery_date.is_none()
            && self.strike.is_none()
    }
}

impl fmt::Display for TradeDifference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "(no changes)");
        }

        // Helper macro: emit "  field_name : before -> after\n" only when Some.
        macro_rules! show {
            ($field:expr, $name:literal) => {
                if let Some((before, after)) = &$field {
                    writeln!(f, "  {:20} {} -> {}", $name, before, after)?;
                }
            };
        }

        show!(self.trading_entity, "trading_entity:");
        show!(self.counterparty, "counterparty:");
        show!(self.direction, "direction:");
        show!(self.style, "style:");
        show!(self.notional_currency, "notional_currency:");
        show!(self.notional_amount, "notional_amount:");
        show!(self.underlying, "underlying:");
        show!(self.trade_date, "trade_date:");
        show!(self.value_date, "value_date:");
        show!(self.delivery_date, "delivery_date:");

        if let Some((before, after)) = &self.strike {
            let b = before.as_deref().unwrap_or("None");
            let a = after.as_deref().unwrap_or("None");
            writeln!(f, "  {:20} {} -> {}", "strike:", b, a)?;
        }

        Ok(())
    }
}
