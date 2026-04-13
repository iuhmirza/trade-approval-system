use std::fmt;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct UserId(u64);

impl UserId {
    pub fn new(id: u64) -> UserId {
        UserId(id)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User({})", self.0)
    }
}
