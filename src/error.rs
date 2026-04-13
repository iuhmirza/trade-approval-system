#[derive(Debug, PartialEq, Eq)]
pub enum TradeError {
    NotValid,
    NotFound,
    NotAuthorized,
}
