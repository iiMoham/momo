use crate::api::schema::{
    Method, Request, WorktreeCreateParams, WorktreeListParams, WorktreeOpenParams,
    WorktreeRemovalCheckParams, WorktreeRemoveDiscardingNestedParams, WorktreeRemoveParams,
};

// Worktree output is always JSON. The parsers retain `--json` as a hidden compatibility no-op.
pub(super) fn run_worktree_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_worktree_help();
        return Ok(2);
    };

    match subcommand {
        "list" => worktree_list(&args[1..]),
        "create" => worktree_create(&args[1..]),
        "open" => worktree_open(&args[1..]),
        "remove" => worktree_remove(&args[1..]),
        "removal-check" => worktree_removal_check(&args[1..]),
        "help" | "--help" | "-h" => {
            print_worktree_help();
            Ok(0)
        }
        _ => {
            print_worktree_help();
            Ok(2)
        }
    }
}

fn worktree_list(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut trust_repository = false;

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
                cwd = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }
    if workspace_id.is_some() && cwd.is_some() {
        eprintln!("usage: momo worktree list [--workspace ID | --cwd PATH] [--trust-repository]");
        return Ok(2);
    }

    super::runtime::worktree_list(WorktreeListParams {
        workspace_id,
        cwd,
        trust_repository,
    })
}

fn worktree_create(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut branch = None;
    let mut base = None;
    let mut path = None;
    let mut label = None;
    let mut focus = false;
    let mut trust_repository = false;

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
                cwd = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--branch" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --branch");
                    return Ok(2);
                };
                branch = Some(value.clone());
                index += 2;
            }
            "--base" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --base");
                    return Ok(2);
                };
                base = Some(value.clone());
                index += 2;
            }
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --path");
                    return Ok(2);
                };
                path = Some(normalize_path_arg(value)?);
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
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }
    if workspace_id.is_some() && cwd.is_some() {
        eprintln!(
            "usage: momo worktree create [--workspace ID | --cwd PATH] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
        );
        return Ok(2);
    }

    super::runtime::worktree_create(WorktreeCreateParams {
        workspace_id,
        cwd,
        branch,
        base,
        path,
        label,
        focus,
        trust_repository,
    })
}

fn worktree_open(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut cwd = None;
    let mut path = None;
    let mut branch = None;
    let mut label = None;
    let mut focus = false;
    let mut trust_repository = false;

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
                cwd = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --path");
                    return Ok(2);
                };
                path = Some(normalize_path_arg(value)?);
                index += 2;
            }
            "--branch" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for --branch");
                    return Ok(2);
                };
                branch = Some(value.clone());
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
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }
    if workspace_id.is_some() && cwd.is_some() {
        eprintln!(
            "usage: momo worktree open [--workspace ID | --cwd PATH] (--path PATH | --branch NAME) [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
        );
        return Ok(2);
    }
    if path.is_some() == branch.is_some() {
        eprintln!(
            "usage: momo worktree open [--workspace ID | --cwd PATH] (--path PATH | --branch NAME) [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
        );
        return Ok(2);
    }

    super::runtime::worktree_open(WorktreeOpenParams {
        workspace_id,
        cwd,
        path,
        branch,
        label,
        focus,
        trust_repository,
    })
}

fn worktree_remove(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut force = false;
    let mut discard_nested = false;
    let mut trust_repository = false;

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
            "--force" => {
                force = true;
                index += 1;
            }
            "--discard-nested" => {
                discard_nested = true;
                index += 1;
            }
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    let Some(workspace_id) = workspace_id else {
        eprintln!("{WORKTREE_REMOVE_USAGE}");
        return Ok(2);
    };

    let params = WorktreeRemoveParams {
        workspace_id,
        force,
        trust_repository,
    };
    if discard_nested {
        return worktree_remove_discarding_nested(params);
    }
    super::runtime::worktree_remove(params)
}

const WORKTREE_REMOVE_USAGE: &str =
    "usage: momo worktree remove --workspace ID [--force] [--discard-nested] [--trust-repository]";

/// Check first, show exactly what will be discarded, then remove while
/// acknowledging only those repositories. Work that appears between the two
/// steps is refused by the server rather than deleted.
fn worktree_remove_discarding_nested(params: WorktreeRemoveParams) -> std::io::Result<i32> {
    let check = super::send_request(&Request {
        id: "cli:worktree:removal-check".into(),
        method: Method::WorktreeRemovalCheck(WorktreeRemovalCheckParams {
            workspace_id: params.workspace_id.clone(),
            trust_repository: params.trust_repository,
        }),
    })?;
    if check.get("error").is_some() {
        return super::print_response(&check);
    }
    let nested = check["result"]["check"]["nested"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut nested_paths = Vec::new();
    for repository in &nested {
        let path = repository["path"].as_str().unwrap_or_default().to_owned();
        eprintln!(
            "discarding {}: {}",
            repository["relative_path"].as_str().unwrap_or(&path),
            repository["summary"].as_str().unwrap_or_default()
        );
        nested_paths.push(path);
    }
    if check["result"]["check"]["complete"] == false {
        eprintln!("the checkout is too large to verify completely; removing anyway");
    }
    super::print_response(&super::send_request(&Request {
        id: "cli:worktree:remove".into(),
        method: Method::WorktreeRemoveDiscardingNested(WorktreeRemoveDiscardingNestedParams {
            workspace_id: params.workspace_id,
            force: params.force,
            trust_repository: params.trust_repository,
            nested_paths,
        }),
    })?)
}

fn worktree_removal_check(args: &[String]) -> std::io::Result<i32> {
    let mut workspace_id = None;
    let mut trust_repository = false;

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
            "--trust-repository" => {
                trust_repository = true;
                index += 1;
            }
            "--json" => index += 1,
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }
    let Some(workspace_id) = workspace_id else {
        eprintln!("usage: momo worktree removal-check --workspace ID [--trust-repository]");
        return Ok(2);
    };
    super::print_response(&super::send_request(&Request {
        id: "cli:worktree:removal-check".into(),
        method: Method::WorktreeRemovalCheck(WorktreeRemovalCheckParams {
            workspace_id,
            trust_repository,
        }),
    })?)
}

fn print_worktree_help() {
    eprintln!("momo worktree commands:");
    eprintln!("  momo worktree list [--workspace ID | --cwd PATH] [--trust-repository]");
    eprintln!(
        "  momo worktree create [--workspace ID | --cwd PATH] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
    );
    eprintln!(
        "  momo worktree open [--workspace ID | --cwd PATH] (--path PATH | --branch NAME) [--label TEXT] [--focus] [--no-focus] [--trust-repository]"
    );
    eprintln!(
        "  momo worktree remove --workspace ID [--force] [--discard-nested] [--trust-repository]"
    );
    eprintln!("  momo worktree removal-check --workspace ID [--trust-repository]");
}

fn normalize_path_arg(value: &str) -> std::io::Result<String> {
    if super::target::is_remote() {
        if super::target::remote_path_is_absolute(value) || value == "~" || value.starts_with("~/")
        {
            return Ok(value.to_owned());
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "remote worktree paths must be absolute or start with ~/",
        ));
    }
    let path = crate::worktree::expand_tilde_path(value);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute.display().to_string())
}

#[cfg(test)]
mod machine_tests {
    #[test]
    fn remote_worktree_paths_are_not_expanded_on_the_caller_machine() {
        crate::cli::target::with_test_client(crate::api::client::ApiClient::local(), || {
            for path in [
                "~/Projects/herdr",
                "/Users/can/Projects/herdr",
                r"C:\work\repo",
                "C:/work/repo",
                r"\\host\share\repo",
            ] {
                assert_eq!(super::normalize_path_arg(path).unwrap(), path);
            }
            assert!(super::normalize_path_arg("../other").is_err());
            assert!(super::normalize_path_arg("C:relative").is_err());
        });
    }
}
