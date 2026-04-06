enum Direction {
    Buy,
    Sell,
}

enum NotionalCurrency {
    GBP,
    USD,
    EUR,
}

enum TradeState {
    Draft,
    PendingApproval,
    NeedsReapproval,
    Approved,
    SendToCounterparty,
    Executed,
    Cancelled
}

struct TradeDetails {
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
    trade_state: TradeState,
}

struct ExecutionConfirmation {}

fn submit_trade_request(user_id: u64, trade_details: TradeDetails) {}

fn approve_trade_request(user_id: u64) {}

fn cancel_trade_request(user_id: u64) {}

fn update_trade_details(user_id: u64, trade_details: TradeDetails) {}

fn send_to_execute(user_id: u64) {}

fn book_executed_trade(user_id: u64, execution_confirmation: ExecutionConfirmation) {}