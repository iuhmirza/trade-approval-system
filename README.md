# Trade Approval System

A Rust library implementing a structured workflow for submitting, approving, and executing financial forward contracts. Data is held in memory; the library is intended as a prototype backend.

---

## Getting Started

```bash
cargo build
cargo test
```

---

## API Reference

All operations are exposed through `TradeRegistry`.

### `TradeRegistry::new() -> TradeRegistry`
Creates an empty registry.

---

### `create_trade(requester_id: UserId, details: TradeDetails) -> Result<TradeId, TradeError>`
Validates `details` and creates a new trade in the `Draft` state. Assigns version `1` to the details.

**Errors:** `NotValid` if validation fails.

---

### `submit(trade_id, user_id, notes) -> Result<(), TradeError>`
Transitions a trade from `Draft` to `PendingApproval`.

**Errors:** `NotFound` · `NotValid` if the trade is not in `Draft`.

---

### `accept(trade_id, user_id, notes, details: Option<TradeDetails>) -> Result<(), TradeError>`
Called by the approver on a `PendingApproval` trade.

- `None` — approves as-is, transitions to `Approved`.
- `Some(details)` — updates the trade details and transitions to `NeedsReapproval`. The updated details are validated and assigned the next version number.

**Errors:** `NotFound` · `NotAuthorized` if the caller is the original requester · `NotValid` if the trade is not in `PendingApproval`, or if updated details fail validation.

---

### `approve(trade_id, user_id, notes) -> Result<(), TradeError>`
Transitions a trade from `NeedsReapproval` to `Approved`. Only the original requester may call this.

**Errors:** `NotFound` · `NotAuthorized` if the caller is not the requester · `NotValid` if the trade is not in `NeedsReapproval`.

---

### `cancel(trade_id, user_id, notes) -> Result<(), TradeError>`
Cancels a trade from any non-terminal state (`Draft`, `PendingApproval`, `NeedsReapproval`, `Approved`, `SendToCounterparty`).

**Errors:** `NotFound` · `NotValid` if the trade is already `Executed` or `Cancelled`.

---

### `send_to_execute(trade_id, user_id, notes) -> Result<(), TradeError>`
Transitions an `Approved` trade to `SendToCounterparty`.

**Errors:** `NotFound` · `NotValid` if the trade is not `Approved`.

---

### `book(trade_id, user_id, notes, strike: String) -> Result<(), TradeError>`
Transitions a `SendToCounterparty` trade to `Executed` and records the agreed strike rate.

**Errors:** `NotFound` · `NotValid` if the trade is not in `SendToCounterparty`.

---

### `get_trade(trade_id) -> Result<&Trade, TradeError>`
Returns a reference to the trade.

---

### `get_history(trade_id) -> Result<&Vec<History>, TradeError>`
Returns the full audit trail. Each `History` entry records: `id`, `user_id`, `state_before`, `state_after`, `details_before`, `notes`, and `timestamp`.

---

### `get_history_at(trade_id, history_id) -> Result<&History, TradeError>`
Returns a single history entry by its ID. Use this to retrieve the trade details that were in effect at a specific transition.

---

### `diff_version(trade_id, v1: TradeVersion, v2: TradeVersion) -> Result<(&TradeDetails, &TradeDetails), TradeError>`
Returns the two `TradeDetails` snapshots for the given version numbers. Call `.diff()` on the returned pair to compute field-level changes.

---

### `TradeDetails::diff(other: &TradeDetails) -> TradeDifference`
Computes a field-level diff. Each field on `TradeDifference` is `None` if unchanged, or `Some((before, after))` if it differed. `TradeDifference` implements `Display`, printing only the changed fields.

---

## Validation Rules

A `TradeDetails` must satisfy all of the following or `NotValid` is returned:

- `trade_date ≤ value_date ≤ delivery_date`
- `trading_entity` and `counterparty` must be non-empty
- `notional_amount` must be greater than zero
- The `notional_currency` (e.g. `GBP`) must appear as a substring of `underlying` (e.g. `GBPUSD`)

---

## Example Scenarios

### 1. Straight-through approval and execution

```rust
let mut registry = TradeRegistry::new();
let user1 = UserId::new(1);
let user2 = UserId::new(2);

let id = registry.create_trade(user1, details)?;
registry.submit(id, user1, "Trade details provided.".to_string())?;
registry.accept(id, user2, "Approver confirms trade.".to_string(), None)?;
registry.send_to_execute(id, user2, "Sent to counterparty.".to_string())?;
registry.book(id, user1, "Executed and booked.".to_string(), "1.3001".to_string())?;

assert_eq!(registry.get_trade(id)?.state, TradeState::Executed);
```

### 2. Approver updates trade details, requester reapproves

```rust
let id = registry.create_trade(user1, details)?;
registry.submit(id, user1, "Trade details provided.".to_string())?;

let mut updated = details.clone();
updated.notional_amount = 1_200_000;
registry.accept(id, user2, "Notional updated.".to_string(), Some(updated))?;

registry.approve(id, user1, "Reapproved.".to_string())?;
```

### 3. Viewing history and diffing versions

```rust
let history = registry.get_history(id)?;
for entry in history {
    println!("{} | {} -> {} | {}", entry.user_id, entry.state_before, entry.state_after, entry.notes);
}

let (v1, v2) = registry.diff_version(id, TradeVersion::new(1), TradeVersion::new(2))?;
println!("{}", v1.diff(v2));
```

---

## Error Types

| Variant | Meaning |
|---|---|
| `TradeError::NotFound` | No trade or history entry exists for the given ID |
| `TradeError::NotValid` | Invalid state transition, failed validation, or illegal operation |
| `TradeError::NotAuthorized` | The caller does not have permission for this action |