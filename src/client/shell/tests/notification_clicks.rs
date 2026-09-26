use super::*;
use crate::api::schema::Method;
use crate::client::endpoint::{
    ClientEndpointId, ClientEndpointStatus, ProfileId, SavedSshEndpoint,
};

fn build_machine() -> SavedSshEndpoint {
    SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Build".into(),
        target: "dev@build.example".into(),
        session: "agents".into(),
        enabled: true,
    }
}

/// Local plus one online saved machine, with system notifications enabled.
fn state_with_machine() -> (ClientShellState, ClientEndpointId) {
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.toast_delivery = crate::config::ToastDelivery::System;
    config.toast_delay_seconds = 0;
    let mut state = ClientShellState::new(config);
    let machine = build_machine();
    let machine_id = ClientEndpointId::Ssh(machine.id.clone());
    state.set_endpoint_catalog(&[machine]);
    state.set_endpoint_status(&machine_id, ClientEndpointStatus::Online);
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let mut remote = snapshot();
    remote.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&machine_id, Box::new(remote));
    (state, machine_id)
}

fn endpoint_methods(outcome: &ClientShellInput) -> Vec<&Method> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => Some(&request.method),
            _ => None,
        })
        .collect()
}

fn custom_notification(pane_id: Option<&str>) -> SemanticNotification {
    SemanticNotification {
        kind: SemanticNotificationKind::Custom,
        title: "agent finished".into(),
        body: Some("$(touch /tmp/pwned)".into()),
        sound: None,
        agent: None,
        workspace_id: Some("ws_1".into()),
        tab_id: Some("tab_1".into()),
        pane_id: pane_id.map(str::to_owned),
        position: None,
    }
}

#[test]
fn system_notifications_with_a_pane_get_a_click_token() {
    let (mut state, machine_id) = state_with_machine();
    // Delivered while the host terminal is unfocused, so it is not suppressed.
    state.outer_focused = Some(false);
    let now = std::time::Instant::now();
    let mut effects = Vec::new();
    for pane in [Some("pane_1"), None] {
        let (received, _) = state.receive_notification(&machine_id, custom_notification(pane), now);
        effects.extend(received);
    }
    effects.extend(state.tick_notifications(now).0);
    let tokens = effects
        .iter()
        .filter_map(|effect| match effect {
            ClientShellNotificationEffect::System { click_token, .. } => Some(click_token.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tokens.len(), 2, "both notifications reach the system");
    assert!(tokens.iter().filter(|token| token.is_some()).count() == 1);
    let token = tokens.into_iter().flatten().next().unwrap();
    assert!(crate::client::notification_click::valid_token(&token));
    assert_eq!(state.notification_clicks.len(), 1);
    assert_eq!(state.notification_clicks[0].pane_id, "pane_1");
    assert_eq!(state.notification_clicks[0].endpoint_id, machine_id);
}

#[test]
fn clicking_opens_the_pane_on_the_active_machine() {
    let (mut state, _) = state_with_machine();
    let token = state.remember_notification_click(ClientEndpointId::Local, "pane_1".into());

    let outcome = state.open_notification_click(&token);
    assert!(matches!(
        endpoint_methods(&outcome)[..],
        [Method::PaneFocus(ref target)] if target.pane_id == "pane_1"
    ));
    // A token is used once.
    let again = state.open_notification_click(&token);
    assert!(again.actions.is_empty());
}

#[test]
fn clicking_switches_to_the_machine_that_raised_the_notification() {
    let (mut state, machine_id) = state_with_machine();
    let token = state.remember_notification_click(machine_id.clone(), "pane_9".into());

    let outcome = state.open_notification_click(&token);
    assert!(matches!(
        &outcome.actions[..],
        [ClientShellAction::ActivateEndpoint {
            endpoint_id,
            target: Some(ClientEndpointFocusTarget::Pane(pane)),
        }] if *endpoint_id == machine_id && pane == "pane_9"
    ));
}

#[test]
fn clicking_a_notification_from_an_offline_machine_reports_it() {
    let (mut state, machine_id) = state_with_machine();
    let token = state.remember_notification_click(machine_id.clone(), "pane_9".into());
    state.set_endpoint_status(&machine_id, ClientEndpointStatus::Reconnecting);

    let outcome = state.open_notification_click(&token);
    assert!(outcome.actions.is_empty());
    assert!(state
        .visible_endpoint_notice
        .as_ref()
        .is_some_and(|notice| notice.body.contains("Build is unavailable")));
}

#[test]
fn unknown_and_expired_tokens_do_nothing() {
    let (mut state, _) = state_with_machine();
    assert!(state.open_notification_click("nope").actions.is_empty());

    let first = state.remember_notification_click(ClientEndpointId::Local, "pane_1".into());
    for _ in 0..crate::client::shell::state::MAX_NOTIFICATION_CLICKS {
        state.remember_notification_click(ClientEndpointId::Local, "pane_1".into());
    }
    assert_eq!(
        state.notification_clicks.len(),
        crate::client::shell::state::MAX_NOTIFICATION_CLICKS
    );
    assert!(state.open_notification_click(&first).actions.is_empty());
}
