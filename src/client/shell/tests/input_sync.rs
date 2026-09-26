use super::*;
use crate::api::schema::{Method, TabInputSyncMode};
use crossterm::event::{MouseButton, MouseEventKind};

fn sync_state(input_sync: bool) -> ClientShellState {
    let mut projected = snapshot();
    projected.tabs[0].input_sync = input_sync;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn input_sync_requests(outcome: &ClientShellInput) -> Vec<(String, TabInputSyncMode)> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => match &request.method {
                Method::TabSetInputSync(params) => Some((params.tab_id.clone(), params.mode)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn tab_menu_labels(state: &ClientShellState) -> Vec<&'static str> {
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("tab context menu should be open");
    };
    menu.items().iter().map(|item| item.label).collect()
}

#[test]
fn synced_tab_shows_s_marker_in_the_warning_color() {
    let mut state = sync_state(true);
    let frame = state.compose(106, 24).unwrap();
    let (tab_rect, _) = state.hits.tabs[0].clone();
    let (x, y) = cell_symbol_position(&frame, tab_rect, "1 S");
    let buffer = frame.to_ratatui_buffer().expect("tab bar buffer");
    let marker = buffer.cell((x + 2, y)).expect("sync marker cell");
    assert_eq!(marker.symbol(), "S");
    assert_eq!(marker.bg, state.config.palette.peach);

    let mut plain = sync_state(false);
    let frame = plain.compose(106, 24).unwrap();
    assert!(!frame_rows(&frame)[0].contains("1 S"));
}

#[test]
fn tab_menu_toggles_input_sync_for_that_tab() {
    for (synced, label) in [(false, "Sync input"), (true, "Stop syncing input")] {
        let mut state = sync_state(synced);
        state.compose(106, 24).unwrap();
        state.open_tab_context_menu("tab_1".into(), 30, 1);
        state.compose(106, 24).unwrap();
        let labels = tab_menu_labels(&state);
        let row = labels.iter().position(|item| *item == label).unwrap();
        let rect = state.hits.context_menu_rows[row].0;
        let outcome =
            state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: rect.x,
                row: rect.y,
                modifiers: KeyModifiers::empty(),
            })]);
        assert_eq!(
            input_sync_requests(&outcome),
            vec![("tab_1".to_owned(), TabInputSyncMode::Toggle)]
        );
    }
}

#[test]
fn tab_menu_hides_input_sync_when_the_server_cannot_toggle_it() {
    let mut state = sync_state(false);
    state.set_endpoint_methods(Some(vec!["tab.close".into(), "tab.focus".into()]));
    state.compose(106, 24).unwrap();
    state.open_tab_context_menu("tab_1".into(), 30, 1);
    let labels = tab_menu_labels(&state);
    assert!(!labels.contains(&"Sync input"));
    assert_eq!(labels, vec!["New tab", "Rename", "Close"]);
}

#[test]
fn toggle_input_sync_binding_targets_the_focused_tab() {
    let mut state = sync_state(false);
    state.compose(106, 24).unwrap();
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::ToggleInputSync),
        &mut outcome,
    );
    assert_eq!(
        input_sync_requests(&outcome),
        vec![("tab_1".to_owned(), TabInputSyncMode::Toggle)]
    );
}

#[test]
fn toggle_input_sync_binding_explains_an_unsupported_server() {
    let mut state = sync_state(false);
    state.set_endpoint_methods(Some(vec!["tab.close".into()]));
    state.compose(106, 24).unwrap();
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::ToggleInputSync),
        &mut outcome,
    );
    assert!(input_sync_requests(&outcome).is_empty());
    let frame = state.compose(106, 24).unwrap();
    assert!(frame_rows(&frame)
        .iter()
        .any(|row| row.contains("Action unavailable")));
}

#[test]
fn mobile_header_mentions_input_sync() {
    let mut state = sync_state(true);
    let frame = state.compose(44, 30).unwrap();
    assert!(frame_rows(&frame)
        .iter()
        .take(2)
        .any(|row| row.contains("sync")));
}
