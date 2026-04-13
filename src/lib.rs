mod error;
mod history;
mod registry;
mod trade;
mod user;

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::{
        error::TradeError,
        history::HistoryId,
        registry::TradeRegistry,
        trade::{Direction, NotionalCurrency, TradeDetails, TradeState},
        user::UserId,
    };

    fn user1() -> UserId {
        UserId::new(1)
    }
    fn user2() -> UserId {
        UserId::new(2)
    }

    fn sample_details() -> TradeDetails {
        TradeDetails::new(
            "EntityA".to_string(),
            "CounterpartyB".to_string(),
            Direction::Buy,
            "Forward".to_string(),
            NotionalCurrency::GBP,
            1_000_000,
            "GBPUSD".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        )
    }

    fn make_pending(registry: &mut TradeRegistry) -> crate::trade::TradeId {
        let id = registry.create_trade(user1(), sample_details()).unwrap();
        registry.submit(id, user1(), "Submit".to_string()).unwrap();
        id
    }

    fn make_approved(registry: &mut TradeRegistry) -> crate::trade::TradeId {
        let id = make_pending(registry);
        registry
            .accept(id, user2(), "Accept".to_string(), None)
            .unwrap();
        id
    }

    #[test]
    fn validation_rejects_inverted_dates() {
        let mut d = sample_details();
        d.value_date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        assert_eq!(d.validate(), Err(TradeError::NotValid));
    }

    #[test]
    fn validation_rejects_empty_trading_entity() {
        let mut d = sample_details();
        d.trading_entity = String::new();
        assert_eq!(d.validate(), Err(TradeError::NotValid));
    }

    #[test]
    fn validation_rejects_empty_counterparty() {
        let mut d = sample_details();
        d.counterparty = String::new();
        assert_eq!(d.validate(), Err(TradeError::NotValid));
    }

    #[test]
    fn validation_rejects_zero_notional() {
        let mut d = sample_details();
        d.notional_amount = 0;
        assert_eq!(d.validate(), Err(TradeError::NotValid));
    }

    #[test]
    fn validation_rejects_currency_not_in_underlying() {
        let mut d = sample_details();

        d.underlying = "EURUSD".to_string();
        assert_eq!(d.validate(), Err(TradeError::NotValid));
    }

    #[test]
    fn validation_accepts_valid_details() {
        assert!(sample_details().validate().is_ok());
    }

    #[test]
    fn submit_moves_draft_to_pending() {
        let mut r = TradeRegistry::new();
        let id = r.create_trade(user1(), sample_details()).unwrap();
        r.submit(id, user1(), String::new()).unwrap();
        assert_eq!(r.get_trade(id).unwrap().state, TradeState::PendingApproval);
    }

    #[test]
    fn accept_without_changes_moves_to_approved() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        r.accept(id, user2(), String::new(), None).unwrap();
        assert_eq!(r.get_trade(id).unwrap().state, TradeState::Approved);
    }

    #[test]
    fn accept_with_changes_moves_to_needs_reapproval() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        let mut updated = sample_details();
        updated.notional_amount = 1_200_000;
        r.accept(id, user2(), String::new(), Some(updated)).unwrap();
        assert_eq!(r.get_trade(id).unwrap().state, TradeState::NeedsReapproval);
    }

    #[test]
    fn approve_by_requester_moves_to_approved() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        let mut updated = sample_details();
        updated.notional_amount = 1_200_000;
        r.accept(id, user2(), String::new(), Some(updated)).unwrap();
        r.approve(id, user1(), String::new()).unwrap();
        assert_eq!(r.get_trade(id).unwrap().state, TradeState::Approved);
    }

    #[test]
    fn send_to_execute_moves_approved_to_sent() {
        let mut r = TradeRegistry::new();
        let id = make_approved(&mut r);
        r.send_to_execute(id, user2(), String::new()).unwrap();
        assert_eq!(
            r.get_trade(id).unwrap().state,
            TradeState::SendToCounterparty
        );
    }

    #[test]
    fn book_moves_sent_to_executed_and_sets_strike() {
        let mut r = TradeRegistry::new();
        let id = make_approved(&mut r);
        r.send_to_execute(id, user2(), String::new()).unwrap();
        r.book(id, user1(), String::new(), "1.2500".to_string())
            .unwrap();
        let trade = r.get_trade(id).unwrap();
        assert_eq!(trade.state, TradeState::Executed);
        assert_eq!(trade.details.strike, Some("1.2500".to_string()));
    }

    #[test]
    fn cancel_from_pending_moves_to_cancelled() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        r.cancel(id, user1(), "Changed mind".to_string()).unwrap();
        assert_eq!(r.get_trade(id).unwrap().state, TradeState::Cancelled);
    }

    #[test]
    fn cannot_submit_from_pending() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        assert_eq!(
            r.submit(id, user1(), String::new()),
            Err(TradeError::NotValid)
        );
    }

    #[test]
    fn requester_cannot_approve_own_trade() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        assert_eq!(
            r.accept(id, user1(), String::new(), None),
            Err(TradeError::NotAuthorized)
        );
    }

    #[test]
    fn only_requester_can_reapprove() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        let mut updated = sample_details();
        updated.notional_amount = 1_200_000;
        r.accept(id, user2(), String::new(), Some(updated)).unwrap();
        assert_eq!(
            r.approve(id, user2(), String::new()),
            Err(TradeError::NotAuthorized)
        );
    }

    #[test]
    fn cannot_cancel_executed_trade() {
        let mut r = TradeRegistry::new();
        let id = make_approved(&mut r);
        r.send_to_execute(id, user2(), String::new()).unwrap();
        r.book(id, user1(), String::new(), "1.25".to_string())
            .unwrap();
        assert_eq!(
            r.cancel(id, user1(), String::new()),
            Err(TradeError::NotValid)
        );
    }

    #[test]
    fn cannot_cancel_already_cancelled_trade() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        r.cancel(id, user1(), String::new()).unwrap();
        assert_eq!(
            r.cancel(id, user1(), String::new()),
            Err(TradeError::NotValid)
        );
    }

    #[test]
    fn accept_rejects_invalid_updated_details() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        let mut bad = sample_details();
        bad.notional_amount = 0;
        assert_eq!(
            r.accept(id, user2(), String::new(), Some(bad)),
            Err(TradeError::NotValid)
        );
    }

    #[test]
    fn get_trade_returns_not_found_for_unknown_id() {
        let r = TradeRegistry::new();
        use crate::trade::TradeId;
        assert_eq!(r.get_trade(TradeId::new(999)), Err(TradeError::NotFound));
    }

    #[test]
    fn history_entry_count_matches_transitions() {
        let mut r = TradeRegistry::new();
        let id = r.create_trade(user1(), sample_details()).unwrap();
        assert_eq!(r.get_history(id).unwrap().len(), 0);

        r.submit(id, user1(), String::new()).unwrap();
        assert_eq!(r.get_history(id).unwrap().len(), 1);

        r.accept(id, user2(), String::new(), None).unwrap();
        assert_eq!(r.get_history(id).unwrap().len(), 2);

        r.send_to_execute(id, user2(), String::new()).unwrap();
        r.book(id, user1(), String::new(), "1.25".to_string())
            .unwrap();
        assert_eq!(r.get_history(id).unwrap().len(), 4);
    }

    #[test]
    fn history_records_correct_states() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        let history = r.get_history(id).unwrap();
        assert_eq!(history[0].state_before, TradeState::Draft);
        assert_eq!(history[0].state_after, TradeState::PendingApproval);
    }

    #[test]
    fn get_details_at_returns_not_found_for_missing_id() {
        let mut r = TradeRegistry::new();
        let id = make_pending(&mut r);
        assert_eq!(
            r.get_history_at(id, HistoryId::new(99)),
            Err(TradeError::NotFound)
        );
    }

    #[test]
    fn scenario_submit_approve_execute_book() {
        let mut r = TradeRegistry::new();
        let id = r.create_trade(user1(), sample_details()).unwrap();
        r.submit(id, user1(), "Trade details provided.".to_string())
            .unwrap();
        r.accept(id, user2(), "Approver confirms trade.".to_string(), None)
            .unwrap();
        r.send_to_execute(id, user2(), "Trade sent to counterparty.".to_string())
            .unwrap();
        r.book(
            id,
            user1(),
            "Trade executed and booked.".to_string(),
            "1.3001".to_string(),
        )
        .unwrap();

        let trade = r.get_trade(id).unwrap();
        assert_eq!(trade.state, TradeState::Executed);
        assert_eq!(r.get_history(id).unwrap().len(), 4);
    }

    #[test]
    fn diff_changed_fields_are_some_unchanged_fields_are_none() {
        let before = sample_details();
        let mut after = sample_details();
        after.notional_amount = 500_000;
        after.counterparty = "CounterpartyZ".to_string();

        let diff = before.diff(&after);

        assert!(!diff.is_empty());
        assert_eq!(diff.notional_amount, Some((1_000_000, 500_000)));
        assert_eq!(
            diff.counterparty,
            Some(("CounterpartyB".to_string(), "CounterpartyZ".to_string()))
        );
        assert!(diff.trading_entity.is_none());
        assert!(diff.direction.is_none());
    }

    #[test]
    fn diff_display_shows_only_changed_fields_and_sentinel_when_empty() {
        // When changes exist: only changed fields appear in the output.
        let before = sample_details();
        let mut after = sample_details();
        after.notional_amount = 1_200_000;

        let output = format!("{}", before.diff(&after));
        assert!(output.contains("notional_amount:"));
        assert!(output.contains("1000000"));
        assert!(output.contains("1200000"));
        assert!(!output.contains("trading_entity:"));

        // When nothing changed: sentinel is shown instead.
        assert_eq!(format!("{}", before.diff(&before)), "(no changes)");
    }
}
