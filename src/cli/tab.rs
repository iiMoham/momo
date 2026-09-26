use std::collections::HashMap;

use crate::api::schema::{
    TabCreateParams, TabInputSyncMode, TabListParams, TabRenameParams, TabSetInputSyncParams,
};

pub(super) fn run_tab_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_tab_help();
        return Ok(2);
    };

    match subcommand {
        "list" => tab_list(&args[1..]),
        "create" => tab_create(&args[1..]),
        "get" => tab_get(&args[1..]),
        "focus" => tab_focus(&args[1..]),
        "rename" => tab_rename(&args[1..]),
        "close" => tab_close(&args[1..]),
        "sync" => tab_sync(&args[1..]),
        "help" | "--help" | "-h" => {
            print_tab_help();
            Ok(0)
        }
        _ => {
            print_tab_help();
            Ok(2)
        }
    }
}

fn tab_list(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --workspace");
                    return Ok(2);
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    super::runtime::tab_list(TabListParams { workspace_id })
}

fn tab_create(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut focus = false;
    let mut label = None;
    let mut env = HashMap::new();

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --workspace");
                    return Ok(2);
                };
                workspace_id = Some(super::normalize_workspace_id(value));
                index += 2;
            }
            "--cwd" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --cwd");
                    return Ok(2);
                };
                cwd = Some(value.clone());
                index += 2;
            }
            "--label" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --label");
                    return Ok(2);
                };
                label = Some(value.clone());
                index += 2;
            }
            "--focus" => {
                focus = true;
                index += 1;
            }
            "--no-focus" => {
                focus = false;
                index += 1;
            }
            "--env" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --env");
                    return Ok(2);
                };
                let (key, value) = match super::parse_env_assignment(value) {
                    Ok(pair) => pair,
                    Err(err) => {
                        eprintln!("{err}");
                        return Ok(2);
                    }
                };
                env.insert(key, value);
                index += 2;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    super::runtime::tab_create(TabCreateParams {
        workspace_id,
        cwd,
        focus,
        label,
        env,
    })
}

fn tab_get(args: &[String]) -> std::io::Result<i32> {
    let Some(raw_tab_id) = args.first() else {
        eprintln!("usage: momo tab get <tab_id>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: momo tab get <tab_id>");
        return Ok(2);
    }

    super::runtime::tab_get(super::normalize_tab_id(raw_tab_id))
}

fn tab_focus(args: &[String]) -> std::io::Result<i32> {
    let Some(raw_tab_id) = args.first() else {
        eprintln!("usage: momo tab focus <tab_id>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: momo tab focus <tab_id>");
        return Ok(2);
    }

    super::runtime::tab_focus(super::normalize_tab_id(raw_tab_id))
}

fn tab_rename(args: &[String]) -> std::io::Result<i32> {
    if args.len() < 2 {
        eprintln!("usage: momo tab rename <tab_id> <label>");
        return Ok(2);
    }

    super::runtime::tab_rename(TabRenameParams {
        tab_id: super::normalize_tab_id(&args[0]),
        label: args[1..].join(" "),
    })
}

fn tab_close(args: &[String]) -> std::io::Result<i32> {
    let Some(raw_tab_id) = args.first() else {
        eprintln!("usage: momo tab close <tab_id>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: momo tab close <tab_id>");
        return Ok(2);
    }

    super::runtime::tab_close(super::normalize_tab_id(raw_tab_id))
}

const TAB_SYNC_USAGE: &str = "usage: momo tab sync <on|off|toggle> [tab_id]";

fn tab_sync(args: &[String]) -> std::io::Result<i32> {
    let mode = match args.first().map(String::as_str) {
        Some("on") => TabInputSyncMode::On,
        Some("off") => TabInputSyncMode::Off,
        Some("toggle") => TabInputSyncMode::Toggle,
        Some(_) | None => {
            eprintln!("{TAB_SYNC_USAGE}");
            return Ok(2);
        }
    };
    if args.len() > 2 {
        eprintln!("{TAB_SYNC_USAGE}");
        return Ok(2);
    }
    // Without an explicit id, target the tab of the pane running the command.
    let Some(raw_tab_id) = args.get(1).cloned().or_else(|| {
        std::env::var("HERDR_TAB_ID")
            .ok()
            .filter(|id| !id.is_empty())
    }) else {
        eprintln!("{TAB_SYNC_USAGE}");
        eprintln!("tab_id is required outside a MoMo pane");
        return Ok(2);
    };

    super::runtime::tab_set_input_sync(TabSetInputSyncParams {
        tab_id: super::normalize_tab_id(&raw_tab_id),
        mode,
    })
}

fn print_tab_help() {
    eprintln!("momo tab commands:");
    eprintln!("  momo tab list [--workspace <workspace_id>]");
    eprintln!(
        "  momo tab create [--workspace <workspace_id>] [--cwd PATH] [--label TEXT] [--env KEY=VALUE] [--focus] [--no-focus]"
    );
    eprintln!("  momo tab get <tab_id>");
    eprintln!("  momo tab focus <tab_id>");
    eprintln!("  momo tab rename <tab_id> <label>");
    eprintln!("  momo tab close <tab_id>");
    eprintln!("  momo tab sync <on|off|toggle> [tab_id]");
}
