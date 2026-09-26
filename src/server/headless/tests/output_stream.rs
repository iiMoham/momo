use super::*;
use crate::api::schema::{
    Method, PaneInfo, PaneReadParams, PaneReadResult, PaneTarget, ReadFormat, ReadSource,
    ResponseResult, SuccessResponse,
};

fn api_result(server: &mut HeadlessServer, method: Method) -> ResponseResult {
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    server.handle_api_request_with_shutdown_check(crate::api::ApiRequestMessage {
        request: crate::api::schema::Request {
            id: "output-stream".into(),
            method,
        },
        respond_to,
        response_write_complete: None,
    });
    let response = response_rx.recv().expect("api response");
    serde_json::from_str::<SuccessResponse>(&response)
        .unwrap_or_else(|_| panic!("expected success, got {response}"))
        .result
}

fn pane_get(server: &mut HeadlessServer, pane_id: &str) -> PaneInfo {
    match api_result(
        server,
        Method::PaneGet(PaneTarget {
            pane_id: pane_id.into(),
        }),
    ) {
        ResponseResult::PaneInfo { pane } => pane,
        other => panic!("expected pane info, got {other:?}"),
    }
}

fn pane_read(server: &mut HeadlessServer, pane_id: &str) -> PaneReadResult {
    match api_result(
        server,
        Method::PaneRead(PaneReadParams {
            pane_id: pane_id.into(),
            source: ReadSource::Visible,
            lines: None,
            format: ReadFormat::Text,
            strip_ansi: true,
            intent: Default::default(),
        }),
    ) {
        ResponseResult::PaneRead { read } => read,
        other => panic!("expected pane read, got {other:?}"),
    }
}

#[tokio::test]
async fn content_revision_tracks_screen_changes_and_matches_pane_read() {
    let mut server = test_headless_server();
    server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("stream")];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    let tab = &server.app.state.workspaces[0].tabs[0];
    let pane = tab.root_pane;
    let terminal_id = tab.terminal_id(pane).unwrap().clone();
    let (runtime, _input_rx) = crate::terminal::TerminalRuntime::test_with_channel(80, 24);
    server
        .app
        .terminal_runtimes
        .insert(terminal_id.clone(), runtime);
    let pane_id = server.app.public_pane_id(0, pane).unwrap();

    let before = pane_get(&mut server, &pane_id)
        .content_revision
        .expect("live pane has a content revision");
    assert_eq!(before % 2, 0, "public revisions are never mid-write");
    assert_eq!(
        pane_get(&mut server, &pane_id).content_revision,
        Some(before)
    );

    server
        .app
        .terminal_runtimes
        .get(&terminal_id)
        .unwrap()
        .test_process_pty_bytes(b"hello stream\r\n");
    let after = pane_get(&mut server, &pane_id).content_revision.unwrap();
    assert!(after > before, "screen output must advance the revision");

    let read = pane_read(&mut server, &pane_id);
    assert_eq!(read.revision, after);
    assert!(read.text.contains("hello stream"));
    shutdown_test_runtimes(&mut server);
}
