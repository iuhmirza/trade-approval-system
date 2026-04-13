use chrono::{DateTime, Utc};

use crate::{
    trade::{TradeDetails, TradeState},
    user::UserId,
};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoryId(u64);

#[derive(Debug, Eq, PartialEq)]
pub struct History {
    pub id: HistoryId,
    pub user_id: UserId,
    pub state_before: TradeState,
    pub state_after: TradeState,
    pub details_before: TradeDetails,
    pub notes: String,
    pub timestamp: DateTime<Utc>,
}

impl History {
    pub fn new(
        id: HistoryId,
        user_id: UserId,
        state_before: TradeState,
        state_after: TradeState,
        details_before: TradeDetails,
        notes: String,
    ) -> History {
        History {
            id,
            user_id,
            state_before,
            state_after,
            details_before,
            timestamp: Utc::now(),
            notes,
        }
    }
}

impl HistoryId {
    pub fn new(id: u64) -> HistoryId {
        HistoryId(id)
    }
}
