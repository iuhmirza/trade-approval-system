use crate::error::TradeError;


pub struct Trade {
    pub id: u64,
    pub requester_id: u64,
    pub state: TradeState,
    pub details: TradeDetails,
    pub history: Vec<Action>,
    next_action_id: u64
}

impl Trade {
    pub fn new(id: u64, requester_id: u64, details: TradeDetails) -> Trade {
        Trade {
            next_action_id: 1,
            id: id,
            requester_id,
            details,
            state: TradeState::Draft,
            history: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct TradeDetails {
    trading_entity: String,
    counterparty: String,
    direction: Direction,
    style: String,
    notional_currency: NotionalCurrency,
    notional_amount: u64,
    underlying: String,
    trade_date: std::time::Instant,
    value_date: std::time::Instant,
    delivery_date: std::time::Instant,
    strike: String,
}

pub enum TradeState {
    Draft,
    PendingApproval,
    NeedsReapproval,
    Approved,
    SendToCounterparty,
    Executed,
    Cancelled
}


#[derive(Clone)]
pub enum Direction {
    Buy,
    Sell,
}

#[derive(Clone)]
pub enum NotionalCurrency {
    GBP,
    USD,
    EUR,
}

pub struct Action {
    id: u64,
    user_id: u64,
    state_before: TradeState,
    state_after: TradeState,
    details_before: TradeDetails,
    notes: String,
}

impl Trade {
    fn push_action(
        &mut self,
        user_id: u64,
        state_before: TradeState,
        state_after: TradeState,
        notes: String
    ) {
        let id = self.next_action_id;
        self.next_action_id += 1;
        self.history.push(Action { 
            id: self.next_action_id, 
            user_id, 
            state_before,
            state_after,
            details_before: self.details.clone(),
            notes
        });
    }
    
    fn submit(&mut self, user_id: u64, notes: String) -> Result<(), TradeError> {
        match self.state {
            TradeState::Draft => {
                self.push_action(user_id, TradeState::Draft, TradeState::PendingApproval, notes);
                self.state = TradeState::PendingApproval;
                Ok(())
            },
            _ => Err(TradeError::NotValid)
        }
    }
    
    fn approve(&mut self, user_id: u64, notes: String) -> Result<(), TradeError> {
        match self.state {
            TradeState::PendingApproval => {
                self.push_action(user_id, TradeState::PendingApproval, TradeState::Approved, notes);
                self.state = TradeState::Approved;
                Ok(())
            },
            TradeState::NeedsReapproval => {
                if self.requester_id != user_id {
                    return Err(TradeError::NotAuthorized);
                }
                self.push_action(user_id, TradeState::NeedsReapproval, TradeState::Approved, notes);
                self.state = TradeState::Approved;
                Ok(())
            },
            _ => Err(TradeError::NotValid),
    }
    }
    
    fn cancel(&mut self, user_id: u64, notes: String) -> Result<(), TradeError> {
        let cancellable = match self.state {
            TradeState::Draft => true,
            TradeState::NeedsReapproval => true,
            TradeState::PendingApproval => true,
            TradeState::Approved => true,
            TradeState::SendToCounterparty => true,
            _=> false
        };
        if !cancellable {
            return Err(TradeError::NotValid);
        }
        let mut state = std::mem::replace(&mut self.state, TradeState::Cancelled);
        self.push_action(user_id, state, TradeState::Cancelled, notes);
        Ok(())
    }
    
    fn update(&mut self) {}
    
    fn send_to_execute(&mut self) {}
    
    fn book(&mut self) {}
}

impl Trade{
    fn history() {}
    fn details() {}
    fn difference() {}
}

impl TradeDetails {
    pub fn validate(&self) -> Result<(), ()> {
        Ok(())
    }
}