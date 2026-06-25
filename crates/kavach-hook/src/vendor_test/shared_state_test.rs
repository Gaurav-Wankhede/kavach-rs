use crate::Vendor;
use kavach_types::HookResponse;

fn next_card_dispatch(card_key: &str) -> HookResponse {
    let mut resp = HookResponse::new_block(&format!(
        "[AUTO_CONTINUE] NEXT TASK [{card_key}]: shared kanban dispatched this card."
    ));
    resp.hook_specific_output = Some(kavach_types::HookSpecificOutput {
        hook_event_name: "Stop".to_owned(),
        ..Default::default()
    });
    resp
}

#[test]
fn same_next_card_reaches_every_vendor_surface() {
    const CARD: &str = "universal.shared-state-proof";
    let verdict = next_card_dispatch(CARD);
    for vendor in Vendor::all() {
        let json = vendor.render_for(&verdict, "Stop");
        assert!(
            json.contains(CARD),
            "{} dropped the shared card key: {json}",
            vendor.name()
        );
        assert!(
            json.contains("AUTO_CONTINUE"),
            "{} dropped the continuation signal: {json}",
            vendor.name()
        );
    }
}

#[test]
fn drained_board_stops_every_vendor_without_resubmit() {
    let drained = HookResponse::new_approve("");
    for vendor in Vendor::all() {
        let json = vendor.render_for(&drained, "Stop");
        assert!(
            !json.contains("followup_message"),
            "{} would resubmit on a drained board: {json}",
            vendor.name()
        );
        assert!(
            !json.contains(r#""decision":"block""#),
            "{} emitted a block on a drained board: {json}",
            vendor.name()
        );
    }
}
