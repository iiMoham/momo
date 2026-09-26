use super::*;
use crate::config::TabBarPositionConfig;
use crossterm::event::{MouseButton, MouseEventKind};

fn padded_state(position: TabBarPositionConfig, padding: u16) -> ClientShellState {
    let mut config = Config::default();
    config.ui.tab_bar_position = position;
    config.ui.tab_bar_padding = padding;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state
}

fn live_position(frame: &FrameData) -> (usize, usize) {
    frame_rows(frame)
        .iter()
        .enumerate()
        .find_map(|(row, text)| {
            text.find("LIVE")
                .map(|byte| (text[..byte].chars().count(), row))
        })
        .expect("pane surface is rendered")
}

#[test]
fn top_tab_bar_padding_leaves_blank_rows_above_panes() {
    for padding in 0..=2u16 {
        let mut state = padded_state(TabBarPositionConfig::Top, padding);
        let frame = state.compose(106, 24).unwrap();
        let rows = frame_rows(&frame);
        let (column, row) = live_position(&frame);
        assert_eq!(row, 1 + usize::from(padding));
        assert!(rows[0].chars().skip(column).any(|symbol| symbol != ' '));
        for text in &rows[1..row] {
            assert!(
                text.chars().skip(column).all(|symbol| symbol == ' '),
                "padding row should be blank: {text:?}"
            );
        }
    }
}

#[test]
fn bottom_tab_bar_padding_sits_between_panes_and_tab_bar() {
    let mut state = padded_state(TabBarPositionConfig::Bottom, 1);
    let frame = state.compose(106, 24).unwrap();
    let rows = frame_rows(&frame);
    let (column, row) = live_position(&frame);
    assert_eq!(row, 0);
    assert!(rows[22].chars().skip(column).all(|symbol| symbol == ' '));
    assert!(rows[23].chars().skip(column).any(|symbol| symbol != ' '));
    assert!(state.hits.tabs.iter().all(|(rect, _)| rect.y == 23));
}

#[test]
fn clicks_in_tab_bar_padding_do_nothing() {
    let mut state = padded_state(TabBarPositionConfig::Top, 2);
    let frame = state.compose(106, 24).unwrap();
    let (column, _) = live_position(&frame);
    for row in [1u16, 2] {
        let outcome =
            state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: column as u16 + 2,
                row,
                modifiers: KeyModifiers::empty(),
            })]);
        assert!(
            outcome.requests.is_empty(),
            "row {row}: {:?}",
            outcome.requests
        );
        assert!(
            outcome.actions.is_empty(),
            "row {row}: {:?}",
            outcome.actions
        );
        assert!(state.overlay.is_none());
    }
}
