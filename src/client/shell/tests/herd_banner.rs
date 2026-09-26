use super::*;
use crossterm::event::{MouseButton, MouseEventKind};

fn banner_state(animation: bool) -> ClientShellState {
    let mut config = Config::default();
    config.ui.animation = animation;
    let mut projected = snapshot();
    projected.agents.push(ClientShellAgent {
        pane_id: "pane_1".into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some("claude".into()),
        display_agent: None,
        agent: Some("claude".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: crate::api::schema::AgentStatus::Working,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: true,
    });
    let mut second = projected.workspaces[0].clone();
    second.workspace_id = "ws_2".into();
    second.number = 2;
    second.label = "second".into();
    second.focused = false;
    projected.workspaces.push(second);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

/// Press and release: workspace rows activate on release.
fn click(state: &mut ClientShellState, rect: Rect) -> ClientShellInput {
    let event = |kind| {
        crate::raw_input::RawInputEvent::Mouse(MouseEvent {
            kind,
            column: rect.x + 1,
            row: rect.y,
            modifiers: KeyModifiers::empty(),
        })
    };
    state.handle_raw_events(vec![
        event(MouseEventKind::Down(MouseButton::Left)),
        event(MouseEventKind::Up(MouseButton::Left)),
    ])
}

#[test]
fn banner_sits_above_the_sidebar_and_clicks_still_hit_the_right_rows() {
    let mut plain = banner_state(false);
    plain.compose(106, 30).unwrap();
    let plain_first_workspace = plain.hits.workspaces[0].rect;

    let mut state = banner_state(true);
    let frame = state.compose(106, 30).unwrap();
    assert_eq!(
        state.hits.herd_banner,
        Rect::new(0, 0, state.hits.herd_banner.width, 4)
    );
    assert!(frame_rows(&frame)[3].contains("MoMo  ● 1 working"));
    let first_workspace = state.hits.workspaces[0].rect;
    assert_eq!(first_workspace.y, plain_first_workspace.y + 4);

    // The banner itself is not a click target.
    let banner_click = click(&mut state, Rect::new(2, 1, 1, 1));
    assert!(banner_click.actions.is_empty());

    let second = state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.workspace_id == "ws_2")
        .unwrap()
        .rect;
    assert!(second.y >= 4, "workspace rows start below the banner");
    let outcome = click(&mut state, second);
    let focused_second = outcome.actions.iter().any(|action| {
        matches!(action, ClientShellAction::Endpoint { request, .. }
            if matches!(&request.method,
                crate::api::schema::Method::WorkspaceFocus(target) if target.workspace_id == "ws_2"))
    });
    assert!(
        focused_second,
        "clicking the shifted row focuses that workspace"
    );
}

#[test]
fn banner_hides_when_off_short_collapsed_or_mobile_and_then_never_wakes_the_timer() {
    let now = std::time::Instant::now();
    let idle_delay = std::time::Duration::from_millis(100);
    let mut cases = Vec::new();

    let mut off = banner_state(false);
    off.compose(106, 30).unwrap();
    cases.push(("off", off));

    let mut short = banner_state(true);
    short.compose(106, 15).unwrap();
    cases.push(("short", short));

    let mut collapsed = banner_state(true);
    collapsed.sidebar_collapsed = true;
    collapsed.compose(106, 30).unwrap();
    cases.push(("collapsed", collapsed));

    let mut mobile = banner_state(true);
    mobile.compose(44, 30).unwrap();
    cases.push(("mobile", mobile));

    for (name, mut state) in cases {
        assert!(state.hits.herd_banner.is_empty(), "{name}");
        assert_eq!(state.timer_delay(now), idle_delay, "{name}");
        assert!(
            !state.tick_animation(now + std::time::Duration::from_secs(5)),
            "{name}"
        );
    }
}

#[test]
fn visible_banner_wakes_once_per_frame_and_repaints_only_on_new_frames() {
    let mut state = banner_state(true);
    state.compose(106, 30).unwrap();
    let interval = state.config.animation_frame_interval;
    assert_eq!(interval, std::time::Duration::from_micros(1_000_000 / 24));
    let epoch = state.animation_epoch;

    assert!(state.timer_delay(epoch) <= interval);
    assert!(!state.tick_animation(epoch), "frame 0 is already drawn");
    assert!(state.tick_animation(epoch + interval));
    assert!(
        !state.tick_animation(epoch + interval + interval / 2),
        "same frame"
    );
    assert!(state.tick_animation(epoch + interval * 2));
    assert_eq!(state.animation_frame, 2);

    // Frames reflect elapsed time: a later frame draws a different herd.
    let before = frame_rows(&state.compose(106, 30).unwrap())[..3].join("\n");
    assert!(state.tick_animation(epoch + std::time::Duration::from_secs(2)));
    let after = frame_rows(&state.compose(106, 30).unwrap())[..3].join("\n");
    assert_ne!(before, after);
}

#[test]
fn animation_settings_parse_clamp_and_reload() {
    let config: Config = toml::from_str("[ui]\nanimation = false\nanimation_fps = 12\n").unwrap();
    assert!(!config.ui.animation);
    assert_eq!(
        config.ui.animation_frame_interval(),
        std::time::Duration::from_micros(1_000_000 / 12)
    );
    for (fps, expected) in [(0, 1), (1, 1), (30, 30), (500, 30)] {
        let config: Config = toml::from_str(&format!("[ui]\nanimation_fps = {fps}\n")).unwrap();
        assert_eq!(
            config.ui.animation_frame_interval(),
            std::time::Duration::from_micros(1_000_000 / expected),
            "fps {fps}"
        );
    }
    assert!(Config::default().ui.animation);

    let fast: Config = toml::from_str("[ui]\nanimation_fps = 500\n").unwrap();
    let mut shell = ClientShellConfig::from_config(&Config::default());
    shell.apply_live_config(&fast, &[], &[]);
    assert_eq!(
        shell.animation_frame_interval,
        std::time::Duration::from_micros(1_000_000 / 30)
    );
    let mut off = Config::default();
    off.ui.animation = false;
    shell.apply_live_config(&off, &[], &[]);
    assert!(!shell.animation);
}
