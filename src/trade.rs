use std::{collections::HashSet, fmt, hash::Hash};

use chrono::NaiveDate;

use crate::{
    error::TradeError,
    history::{History, HistoryId},
    registry::TradeDifference,
    user::UserId,
};

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct TradeId(u64);

#[derive(Debug, Eq, PartialEq)]
pub struct Trade {
    pub id: TradeId,
    pub requester_id: UserId,
    pub state: TradeState,
    pub details: TradeDetails,
    pub history: Vec<History>,
    next_history_id: u64,
}

impl TradeId {
    pub fn new(id: u64) -> TradeId {
        TradeId(id)
    }
}

impl Trade {
    pub fn new(id: TradeId, requester_id: UserId, details: TradeDetails) -> Trade {
        Trade {
            next_history_id: 1,
            id: id,
            requester_id,
            details,
            state: TradeState::Draft,
            history: Vec::new(),
        }
    }

    pub fn next_history_id(&mut self) -> HistoryId {
        let id = self.next_history_id;
        self.next_history_id += 1;
        HistoryId::new(id)
    }
}
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct TradeVersion(u64);

impl TradeVersion {
    pub fn new(version: u64) -> TradeVersion {
        TradeVersion(version)
    }
    pub fn raw(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct TradeDetails {
    pub version: TradeVersion,
    pub trading_entity: String,
    pub counterparty: String,
    pub direction: Direction,
    pub style: String,
    pub notional_currency: NotionalCurrency,
    pub notional_amount: u64,
    pub underlying: String,
    pub trade_date: NaiveDate,
    pub value_date: NaiveDate,
    pub delivery_date: NaiveDate,
    pub strike: Option<String>,
}

impl TradeDetails {
    pub fn diff(&self, other: &TradeDetails) -> TradeDifference {
        macro_rules! changed {
            ($field:ident) => {
                if self.$field != other.$field {
                    Some((self.$field.clone(), other.$field.clone()))
                } else {
                    None
                }
            };
        }

        TradeDifference {
            trading_entity: changed!(trading_entity),
            counterparty: changed!(counterparty),
            direction: changed!(direction),
            style: changed!(style),
            notional_currency: changed!(notional_currency),
            notional_amount: changed!(notional_amount),
            underlying: changed!(underlying),
            trade_date: changed!(trade_date),
            value_date: changed!(value_date),
            delivery_date: changed!(delivery_date),
            strike: changed!(strike),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum TradeState {
    Draft,
    PendingApproval,
    NeedsReapproval,
    Approved,
    SendToCounterparty,
    Executed,
    Cancelled,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum Direction {
    Buy,
    Sell,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum NotionalCurrency {
    GBP,
    USD,
    EUR,
}

impl Trade {
    fn push_history(
        &mut self,
        user_id: UserId,
        state_before: TradeState,
        state_after: TradeState,
        notes: String,
    ) {
        let id = self.next_history_id();
        self.history.push(History::new(
            id,
            user_id,
            state_before,
            state_after,
            self.details.clone(),
            notes,
        ))
    }

    pub fn submit(&mut self, user_id: UserId, notes: String) -> Result<(), TradeError> {
        match self.state {
            TradeState::Draft => {
                self.push_history(
                    user_id,
                    TradeState::Draft,
                    TradeState::PendingApproval,
                    notes,
                );
                self.state = TradeState::PendingApproval;
                Ok(())
            }
            _ => Err(TradeError::NotValid),
        }
    }

    pub fn accept(
        &mut self,
        user_id: UserId,
        notes: String,
        details: Option<TradeDetails>,
    ) -> Result<(), TradeError> {
        if self.requester_id == user_id {
            return Err(TradeError::NotAuthorized);
        }
        match details {
            Some(details) => match details.validate() {
                Ok(_) => match self.state {
                    TradeState::PendingApproval => {
                        self.push_history(
                            user_id,
                            TradeState::PendingApproval,
                            TradeState::NeedsReapproval,
                            notes,
                        );
                        self.state = TradeState::NeedsReapproval;
                        self.details = details;
                        Ok(())
                    }
                    _ => Err(TradeError::NotValid),
                },
                Err(e) => Err(e),
            },
            None => match self.state {
                TradeState::PendingApproval => {
                    self.push_history(
                        user_id,
                        TradeState::PendingApproval,
                        TradeState::Approved,
                        notes,
                    );
                    self.state = TradeState::Approved;
                    Ok(())
                }
                _ => Err(TradeError::NotValid),
            },
        }
    }

    pub fn approve(&mut self, user_id: UserId, notes: String) -> Result<(), TradeError> {
        match self.state {
            TradeState::NeedsReapproval => {
                if self.requester_id != user_id {
                    return Err(TradeError::NotAuthorized);
                }
                self.push_history(
                    user_id,
                    TradeState::NeedsReapproval,
                    TradeState::Approved,
                    notes,
                );
                self.state = TradeState::Approved;
                Ok(())
            }
            _ => Err(TradeError::NotValid),
        }
    }

    pub fn cancel(&mut self, user_id: UserId, notes: String) -> Result<(), TradeError> {
        let cancellable = match self.state {
            TradeState::Draft => true,
            TradeState::NeedsReapproval => true,
            TradeState::PendingApproval => true,
            TradeState::Approved => true,
            TradeState::SendToCounterparty => true,
            _ => false,
        };
        if !cancellable {
            return Err(TradeError::NotValid);
        }
        let state = std::mem::replace(&mut self.state, TradeState::Cancelled);
        self.push_history(user_id, state, TradeState::Cancelled, notes);
        Ok(())
    }

    pub fn send_to_execute(&mut self, user_id: UserId, notes: String) -> Result<(), TradeError> {
        match self.state {
            TradeState::Approved => {
                self.push_history(
                    user_id,
                    TradeState::Approved,
                    TradeState::SendToCounterparty,
                    notes,
                );
                self.state = TradeState::SendToCounterparty;
                Ok(())
            }
            _ => Err(TradeError::NotValid),
        }
    }

    pub fn book(
        &mut self,
        user_id: UserId,
        notes: String,
        strike: String,
    ) -> Result<(), TradeError> {
        match self.state {
            TradeState::SendToCounterparty => {
                self.push_history(
                    user_id,
                    TradeState::SendToCounterparty,
                    TradeState::Executed,
                    notes,
                );
                self.state = TradeState::Executed;
                self.details.strike = Some(strike);
                Ok(())
            }
            _ => Err(TradeError::NotValid),
        }
    }

    pub fn versions(&self) -> Vec<&TradeDetails> {
        let mut set = HashSet::new();
        let mut versions = Vec::new();
        for details in self.history.iter().map(|h| &h.details_before) {
            if !set.contains(&details) {
                set.insert(details);
                versions.push(details);
            }
        }
        if !set.contains(&self.details) {
            versions.push(&self.details);
        }
        versions
    }
}

impl TradeDetails {
    pub fn new(
        trading_entity: String,
        counterparty: String,
        direction: Direction,
        style: String,
        notional_currency: NotionalCurrency,
        notional_amount: u64,
        underlying: String,
        trade_date: NaiveDate,
        value_date: NaiveDate,
        delivery_date: NaiveDate,
    ) -> TradeDetails {
        TradeDetails {
            version: TradeVersion(0),
            trading_entity,
            counterparty,
            direction,
            style,
            notional_currency,
            notional_amount,
            underlying,
            trade_date,
            value_date,
            delivery_date,
            strike: None,
        }
    }

    pub fn validate(&self) -> Result<(), TradeError> {
        if !(self.trade_date <= self.value_date && self.value_date <= self.delivery_date) {
            return Err(TradeError::NotValid);
        }
        if self.trading_entity.is_empty() || self.counterparty.is_empty() {
            return Err(TradeError::NotValid);
        }
        if self.notional_amount == 0 {
            return Err(TradeError::NotValid);
        }
        let currency_str = self.notional_currency.to_string();
        if !self
            .underlying
            .to_uppercase()
            .contains(currency_str.as_str())
        {
            return Err(TradeError::NotValid);
        }
        Ok(())
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Trade({})", self.0)
    }
}

impl fmt::Display for TradeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TradeState::Draft => "Draft",
            TradeState::PendingApproval => "PendingApproval",
            TradeState::NeedsReapproval => "NeedsReapproval",
            TradeState::Approved => "Approved",
            TradeState::SendToCounterparty => "SentToCounterparty",
            TradeState::Executed => "Executed",
            TradeState::Cancelled => "Cancelled",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Buy => write!(f, "Buy"),
            Direction::Sell => write!(f, "Sell"),
        }
    }
}

impl fmt::Display for NotionalCurrency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotionalCurrency::GBP => write!(f, "GBP"),
            NotionalCurrency::USD => write!(f, "USD"),
            NotionalCurrency::EUR => write!(f, "EUR"),
        }
    }
}
