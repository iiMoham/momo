use super::*;
use crate::api::schema::{
    ErrorResponse, Method, PaneSendTextParams, ResponseResult, SuccessResponse, TabInfo,
    TabInputSyncMode, TabSetInputSyncParams,
};
use crate::protocol::ClientPaneInputEvent;

struct SyncedTab {
    server: HeadlessServer,
    focused: String,
    inputs: Vec<(crate::layout::PaneId, tokio::sync::mpsc::Receiver<Bytes>)>,
}

/// One workspace with a three-pane tab plus a second single-pane tab. Every pane in
/// the first tab gets a test runtime whose PTY input is captured.
fn synced_tab_server() -> SyncedTab {
    let mut server = test_headless_server();
    let mut workspace = crate::workspace::Workspace::test_new("sync");
    workspace.test_split(ratatui::layout::Direction::Horizontal);
    workspace.test_split(ratatui::layout::Direction::Horizontal);
    workspace.test_add_tab(Some("other"));
    workspace.switch_tab(0);
    server.app.state.workspaces = vec![workspace];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.mode = crate::app::Mode::Terminal;

    let tab = &server.app.state.workspaces[0].tabs[0];
    let focused_pane = tab.layout.focused();
    let mut inputs = Vec::new();
    for pane in tab.layout.pane_ids() {
        let terminal_id = tab.terminal_id(pane).unwrap().clone();
        let (runtime, input_rx) = crate::terminal::TerminalRuntime::test_with_channel(80, 24);
        server.app.terminal_runtimes.insert(terminal_id, runtime);
        inputs.push((pane, input_rx));
    }
    let focused = server.app.public_pane_id(0, focused_pane).unwrap();
    SyncedTab {
        server,
        focused,
        inputs,
    }
}

fn api_request(server: &mut HeadlessServer, method: Method) -> String {
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    server.handle_api_request_with_shutdown_check(crate::api::ApiRequestMessage {
        request: crate::api::schema::Request {
            id: "input-sync".into(),
            method,
        },
        respond_to,
        response_write_complete: None,
    });
    response_rx.recv().expect("api response")
}

fn set_input_sync(
    server: &mut HeadlessServer,
    tab_id: &str,
    mode: TabInputSyncMode,
) -> Result<TabInfo, ErrorResponse> {
    let response = api_request(
        server,
        Method::TabSetInputSync(TabSetInputSyncParams {
            tab_id: tab_id.into(),
            mode,
        }),
    );
    match serde_json::from_str::<SuccessResponse>(&response) {
        Ok(SuccessResponse {
            result: ResponseResult::TabInfo { tab },
            ..
        }) => Ok(tab),
        Ok(other) => panic!("expected tab info, got {other:?}"),
        Err(_) => Err(serde_json::from_str(&response).expect("error response")),
    }
}

fn type_into(server: &mut HeadlessServer, pane_id: &str, events: Vec<ClientPaneInputEvent>) {
    server.handle_server_event(ServerEvent::ClientShellPaneInput {
        client_id: 9,
        pane_id: pane_id.into(),
        events,
    });
}

fn drain(rx: &mut tokio::sync::mpsc::Receiver<Bytes>) -> Vec<u8> {
    let mut bytes = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        bytes.extend_from_slice(&chunk);
    }
    bytes
}

fn first_tab_id(server: &HeadlessServer) -> String {
    server.app.public_tab_id(0, 0).unwrap()
}

#[tokio::test]
async fn synced_tab_broadcasts_typing_and_paste_to_every_pane() {
    let SyncedTab {
        mut server,
        focused,
        mut inputs,
    } = synced_tab_server();
    let (_control_rx, _render_rx) = connect_test_shell(&mut server, 9, 80, 23);
    let tab_id = first_tab_id(&server);

    let tab = set_input_sync(&mut server, &tab_id, TabInputSyncMode::On).unwrap();
    assert!(tab.input_sync);

    type_into(
        &mut server,
        &focused,
        vec![
            ClientPaneInputEvent::TextCommit("echo hi".into()),
            ClientPaneInputEvent::Paste("pasted".into()),
        ],
    );
    assert_eq!(inputs.len(), 3);
    for (pane, rx) in &mut inputs {
        assert_eq!(drain(rx), b"echo hipasted", "pane {pane:?}");
    }
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn unsynced_tab_types_only_into_the_focused_pane() {
    let SyncedTab {
        mut server,
        focused,
        mut inputs,
    } = synced_tab_server();
    let (_control_rx, _render_rx) = connect_test_shell(&mut server, 9, 80, 23);
    let focused_pane = server.app.state.workspaces[0].tabs[0].layout.focused();

    type_into(
        &mut server,
        &focused,
        vec![ClientPaneInputEvent::TextCommit("x".into())],
    );
    for (pane, rx) in &mut inputs {
        let expected: &[u8] = if *pane == focused_pane { b"x" } else { b"" };
        assert_eq!(drain(rx), expected, "pane {pane:?}");
    }
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn each_synced_pane_encodes_paste_for_its_own_terminal_mode() {
    let SyncedTab {
        mut server,
        focused,
        mut inputs,
    } = synced_tab_server();
    let (_control_rx, _render_rx) = connect_test_shell(&mut server, 9, 80, 23);
    let tab_id = first_tab_id(&server);
    let bracketed = inputs
        .iter()
        .map(|(pane, _)| *pane)
        .find(|pane| server.app.public_pane_id(0, *pane).as_deref() != Some(focused.as_str()))
        .unwrap();
    let terminal_id = server.app.state.workspaces[0].tabs[0]
        .terminal_id(bracketed)
        .unwrap()
        .clone();
    server
        .app
        .terminal_runtimes
        .get(&terminal_id)
        .unwrap()
        .test_process_pty_bytes(b"\x1b[?2004h");
    set_input_sync(&mut server, &tab_id, TabInputSyncMode::On).unwrap();

    type_into(
        &mut server,
        &focused,
        vec![ClientPaneInputEvent::Paste("p".into())],
    );
    for (pane, rx) in &mut inputs {
        let expected: &[u8] = if *pane == bracketed {
            b"\x1b[200~p\x1b[201~"
        } else {
            b"p"
        };
        assert_eq!(drain(rx), expected, "pane {pane:?}");
    }
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn api_input_is_never_broadcast_to_synced_siblings() {
    let SyncedTab {
        mut server,
        focused,
        mut inputs,
    } = synced_tab_server();
    let tab_id = first_tab_id(&server);
    let focused_pane = server.app.state.workspaces[0].tabs[0].layout.focused();
    set_input_sync(&mut server, &tab_id, TabInputSyncMode::On).unwrap();

    api_request(
        &mut server,
        Method::PaneSendText(PaneSendTextParams {
            pane_id: focused.clone(),
            text: "api".into(),
        }),
    );
    for (pane, rx) in &mut inputs {
        let expected: &[u8] = if *pane == focused_pane { b"api" } else { b"" };
        assert_eq!(drain(rx), expected, "pane {pane:?}");
    }
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn a_closed_sibling_does_not_stop_delivery_to_the_others() {
    let SyncedTab {
        mut server,
        focused,
        mut inputs,
    } = synced_tab_server();
    let (_control_rx, _render_rx) = connect_test_shell(&mut server, 9, 80, 23);
    let tab_id = first_tab_id(&server);
    let focused_pane = server.app.state.workspaces[0].tabs[0].layout.focused();
    set_input_sync(&mut server, &tab_id, TabInputSyncMode::On).unwrap();
    // Dropping a receiver closes that pane's input channel, so sending to it fails.
    let closed = inputs
        .iter()
        .position(|(pane, _)| *pane != focused_pane)
        .unwrap();
    inputs.remove(closed);

    type_into(
        &mut server,
        &focused,
        vec![ClientPaneInputEvent::TextCommit("y".into())],
    );
    assert_eq!(inputs.len(), 2);
    for (pane, rx) in &mut inputs {
        assert_eq!(drain(rx), b"y", "pane {pane:?}");
    }
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn input_sync_is_per_tab_toggles_and_reaches_the_client_snapshot() {
    let SyncedTab { mut server, .. } = synced_tab_server();
    let (control_rx, _render_rx) = connect_test_shell(&mut server, 9, 80, 23);
    let initial = client_shell_snapshot(&control_rx);
    assert!(initial.tabs.iter().all(|tab| !tab.input_sync));
    let first = first_tab_id(&server);
    let second = server.app.public_tab_id(0, 1).unwrap();

    assert!(
        set_input_sync(&mut server, &first, TabInputSyncMode::Toggle)
            .unwrap()
            .input_sync
    );
    assert!(!server.app.state.workspaces[0].tabs[1].input_sync);
    server.render_and_stream();
    let snapshot = client_shell_snapshot(&control_rx);
    assert!(snapshot.revision > initial.revision);
    let synced = |id: &str| {
        snapshot
            .tabs
            .iter()
            .find(|tab| tab.tab_id == id)
            .unwrap()
            .input_sync
    };
    assert!(synced(&first));
    assert!(!synced(&second));

    assert!(
        !set_input_sync(&mut server, &first, TabInputSyncMode::Toggle)
            .unwrap()
            .input_sync
    );
    assert!(
        !set_input_sync(&mut server, &first, TabInputSyncMode::Off)
            .unwrap()
            .input_sync
    );
    let missing = set_input_sync(&mut server, "w999:t1", TabInputSyncMode::On).unwrap_err();
    assert_eq!(missing.error.code, "tab_not_found");
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn tab_info_json_only_mentions_input_sync_when_enabled() {
    let SyncedTab { mut server, .. } = synced_tab_server();
    let tab_id = first_tab_id(&server);
    let get = |server: &mut HeadlessServer| {
        api_request(
            server,
            Method::TabGet(crate::api::schema::TabTarget {
                tab_id: tab_id.clone(),
            }),
        )
    };
    assert!(!get(&mut server).contains("input_sync"));
    set_input_sync(&mut server, &tab_id, TabInputSyncMode::On).unwrap();
    assert!(get(&mut server).contains("\"input_sync\":true"));
    shutdown_test_runtimes(&mut server);
}
