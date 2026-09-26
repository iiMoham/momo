//! Pre-removal safety check for worktree checkouts.
//!
//! `git worktree remove` only sees files its own repository tracks. Git
//! repositories nested inside a checkout (sub-repos a hub ignores, submodules,
//! other worktrees) are deleted with the directory even when they hold
//! uncommitted work or commits that exist on no remote. This module finds those
//! repositories so removal can refuse instead of losing work silently.
//!
//! Everything here does filesystem and process I/O: call it only from a
//! background thread, never from the app loop or render paths.

use std::path::{Path, PathBuf};

/// Deepest directory level below the checkout that is searched. Symlinks are not
/// followed, so this only bounds genuinely deep trees; the entry budget bounds cost.
pub(crate) const MAX_SCAN_DEPTH: usize = 32;
/// Directory entries examined before the scan gives up and reports itself incomplete.
pub(crate) const MAX_SCAN_ENTRIES: usize = 200_000;
/// Refusal messages list at most this many repositories.
const MAX_LISTED_REPOSITORIES: usize = 10;
/// Dependency and build directories that never hold user repositories worth protecting.
const SKIPPED_DIRECTORIES: &[&str] = &["node_modules", "target", ".venv", "__pycache__"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NestedRepositoryKind {
    /// Its own `.git` directory: history and stashes live inside the checkout.
    Clone,
    /// A `.git` file whose Git data also lives inside the checkout or its Git dir.
    Submodule,
    /// A worktree of a repository outside the checkout: only working-tree changes are at risk.
    LinkedWorktree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NestedRepositoryRisk {
    pub(crate) path: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) kind: NestedRepositoryKind,
    pub(crate) modified: u32,
    pub(crate) untracked: u32,
    pub(crate) stashes: u32,
    pub(crate) unpushed_commits: u32,
    /// Set when Git could not inspect the repository; treated as at risk.
    pub(crate) inspection_error: Option<String>,
}

impl NestedRepositoryRisk {
    fn at_risk(&self) -> bool {
        self.modified > 0
            || self.untracked > 0
            || self.stashes > 0
            || self.unpushed_commits > 0
            || self.inspection_error.is_some()
    }

    pub(crate) fn to_info(&self) -> crate::api::schema::NestedRepositoryRiskInfo {
        crate::api::schema::NestedRepositoryRiskInfo {
            path: self.path.display().to_string(),
            relative_path: self.relative_path.display().to_string(),
            kind: match self.kind {
                NestedRepositoryKind::Clone => crate::api::schema::NestedRepositoryKind::Clone,
                NestedRepositoryKind::Submodule => {
                    crate::api::schema::NestedRepositoryKind::Submodule
                }
                NestedRepositoryKind::LinkedWorktree => {
                    crate::api::schema::NestedRepositoryKind::LinkedWorktree
                }
            },
            summary: self.summary(),
            modified: self.modified,
            untracked: self.untracked,
            stashes: self.stashes,
            unpushed_commits: self.unpushed_commits,
            inspection_error: self.inspection_error.clone(),
        }
    }

    /// Human summary such as `2 unpushed commits, 1 untracked path`.
    pub(crate) fn summary(&self) -> String {
        if let Some(error) = &self.inspection_error {
            return format!("could not be inspected ({error})");
        }
        let mut parts = Vec::new();
        for (count, singular, plural) in [
            (self.unpushed_commits, "unpushed commit", "unpushed commits"),
            (self.modified, "modified file", "modified files"),
            (self.untracked, "untracked path", "untracked paths"),
            (self.stashes, "stash", "stashes"),
        ] {
            match count {
                0 => {}
                1 => parts.push(format!("1 {singular}")),
                count => parts.push(format!("{count} {plural}")),
            }
        }
        parts.join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemovalCheck {
    /// Nested repositories with something that removal would lose, in path order.
    pub(crate) at_risk: Vec<NestedRepositoryRisk>,
    /// False when a budget or unreadable directory stopped the scan early.
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemovalRefusal {
    NestedRepositoriesAtRisk,
    CheckIncomplete,
}

impl RemovalRefusal {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::NestedRepositoriesAtRisk => "worktree_nested_repositories_at_risk",
            Self::CheckIncomplete => "worktree_removal_check_incomplete",
        }
    }
}

impl RemovalCheck {
    /// Why a plain removal must be refused, if it must.
    pub(crate) fn refusal(&self, checkout: &Path) -> Option<(RemovalRefusal, String)> {
        if !self.at_risk.is_empty() {
            return Some((
                RemovalRefusal::NestedRepositoriesAtRisk,
                self.refusal_message(checkout, &self.at_risk.iter().collect::<Vec<_>>()),
            ));
        }
        if !self.complete {
            return Some((
                RemovalRefusal::CheckIncomplete,
                format!(
                    "refusing to remove {}: the checkout is too large to verify that no nested repository would lose work; remove it with --discard-nested to proceed anyway",
                    checkout.display()
                ),
            ));
        }
        None
    }

    /// Why a removal that acknowledged `acknowledged` repositories must still be
    /// refused: any at-risk repository missing from that list, for example work
    /// that appeared after the user reviewed the check.
    pub(crate) fn unacknowledged_refusal(
        &self,
        checkout: &Path,
        acknowledged: &[PathBuf],
    ) -> Option<(RemovalRefusal, String)> {
        let acknowledged = acknowledged
            .iter()
            .map(|path| crate::worktree::canonical_or_original(path))
            .collect::<Vec<_>>();
        let missing = self
            .at_risk
            .iter()
            .filter(|risk| {
                !acknowledged.contains(&crate::worktree::canonical_or_original(&risk.path))
            })
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return None;
        }
        Some((
            RemovalRefusal::NestedRepositoriesAtRisk,
            self.refusal_message(checkout, &missing),
        ))
    }

    fn refusal_message(&self, checkout: &Path, risks: &[&NestedRepositoryRisk]) -> String {
        let count = risks.len();
        let noun = if count == 1 {
            "nested repository has"
        } else {
            "nested repositories have"
        };
        let mut message = format!(
            "refusing to remove {}: {count} {noun} work that would be lost:",
            checkout.display()
        );
        for risk in risks.iter().take(MAX_LISTED_REPOSITORIES) {
            message.push_str(&format!(
                "\n  {}: {}",
                risk.relative_path.display(),
                risk.summary()
            ));
        }
        if count > MAX_LISTED_REPOSITORIES {
            message.push_str(&format!(
                "\n  …and {} more",
                count - MAX_LISTED_REPOSITORIES
            ));
        }
        if !self.complete {
            message.push_str("\nthe scan stopped early, so more may be at risk");
        }
        message.push_str("\nremove it with --discard-nested to delete this work anyway");
        message
    }
}

/// Find nested Git repositories under `checkout` whose work removal would lose.
pub(crate) fn check_checkout_for_removal(checkout: &Path, trust_repository: bool) -> RemovalCheck {
    check_with_budget(checkout, trust_repository, MAX_SCAN_DEPTH, MAX_SCAN_ENTRIES)
}

fn check_with_budget(
    checkout: &Path,
    trust_repository: bool,
    max_depth: usize,
    max_entries: usize,
) -> RemovalCheck {
    let checkout_canonical = crate::worktree::canonical_or_original(checkout);
    let checkout_git_dir =
        git_dir_of(checkout).map(|dir| crate::worktree::canonical_or_original(&dir));
    let mut at_risk = Vec::new();
    let mut complete = true;
    let mut entries = 0usize;
    let mut stack = vec![(checkout.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            // An unreadable directory might hide a repository.
            complete = false;
            continue;
        };
        for entry in read_dir {
            entries += 1;
            if entries > max_entries {
                complete = false;
                stack.clear();
                break;
            }
            let Ok(entry) = entry else {
                complete = false;
                continue;
            };
            // `DirEntry::file_type` does not follow symlinks; neither does removal.
            let Ok(file_type) = entry.file_type() else {
                complete = false;
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            if name == ".git" || SKIPPED_DIRECTORIES.iter().any(|skip| name == *skip) {
                continue;
            }
            if depth >= max_depth {
                // A directory below the depth limit exists and was not searched.
                complete = false;
                continue;
            }
            let path = entry.path();
            if let Some(kind) =
                nested_repository_kind(&path, &checkout_canonical, checkout_git_dir.as_deref())
            {
                let risk = inspect_repository(checkout, &path, kind, trust_repository);
                if risk.at_risk() {
                    at_risk.push(risk);
                }
            }
            stack.push((path, depth + 1));
        }
    }

    at_risk.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    RemovalCheck { at_risk, complete }
}

/// Git dir named by a `.git` file (`gitdir: …`), or the `.git` directory itself.
fn git_dir_of(repo: &Path) -> Option<PathBuf> {
    let dot_git = repo.join(".git");
    let metadata = std::fs::symlink_metadata(&dot_git).ok()?;
    if metadata.is_dir() {
        return Some(dot_git);
    }
    let content = std::fs::read_to_string(&dot_git).ok()?;
    let gitdir = PathBuf::from(content.trim().strip_prefix("gitdir:")?.trim());
    Some(if gitdir.is_absolute() {
        gitdir
    } else {
        repo.join(gitdir)
    })
}

fn nested_repository_kind(
    path: &Path,
    checkout: &Path,
    checkout_git_dir: Option<&Path>,
) -> Option<NestedRepositoryKind> {
    let metadata = std::fs::symlink_metadata(path.join(".git")).ok()?;
    if metadata.is_dir() {
        return Some(NestedRepositoryKind::Clone);
    }
    let Some(git_dir) = git_dir_of(path) else {
        // A `.git` file we cannot parse: inspect it like a clone rather than skip it.
        return Some(NestedRepositoryKind::Clone);
    };
    let git_dir = crate::worktree::canonical_or_original(&git_dir);
    let data_is_deleted = git_dir.starts_with(checkout)
        || checkout_git_dir.is_some_and(|checkout_git_dir| git_dir.starts_with(checkout_git_dir));
    Some(if data_is_deleted {
        NestedRepositoryKind::Submodule
    } else {
        NestedRepositoryKind::LinkedWorktree
    })
}

fn inspect_repository(
    checkout: &Path,
    repo: &Path,
    kind: NestedRepositoryKind,
    trust_repository: bool,
) -> NestedRepositoryRisk {
    let mut risk = NestedRepositoryRisk {
        path: repo.to_path_buf(),
        relative_path: repo.strip_prefix(checkout).unwrap_or(repo).to_path_buf(),
        kind,
        modified: 0,
        untracked: 0,
        stashes: 0,
        unpushed_commits: 0,
        inspection_error: None,
    };
    match git_output(
        repo,
        trust_repository,
        &["status", "--porcelain=v1", "-unormal"],
    ) {
        Ok(status) => {
            for line in status.lines().filter(|line| !line.is_empty()) {
                if line.starts_with("??") {
                    risk.untracked += 1;
                } else {
                    risk.modified += 1;
                }
            }
        }
        Err(error) => {
            risk.inspection_error = Some(error);
            return risk;
        }
    }
    if kind == NestedRepositoryKind::LinkedWorktree {
        return risk;
    }
    match git_output(repo, trust_repository, &["stash", "list"]) {
        Ok(stashes) => risk.stashes = count_lines(&stashes),
        Err(error) => {
            risk.inspection_error = Some(error);
            return risk;
        }
    }
    // Commits reachable from local branches or HEAD but from no remote-tracking ref.
    // A repository without commits has no HEAD, so retry with branches alone.
    let unpushed = git_output(
        repo,
        trust_repository,
        &[
            "rev-list",
            "--count",
            "--branches",
            "HEAD",
            "--not",
            "--remotes",
        ],
    )
    .or_else(|_| {
        git_output(
            repo,
            trust_repository,
            &["rev-list", "--count", "--branches", "--not", "--remotes"],
        )
    });
    match unpushed {
        Ok(count) => risk.unpushed_commits = count.trim().parse().unwrap_or(0),
        Err(error) => risk.inspection_error = Some(error),
    }
    risk
}

fn count_lines(text: &str) -> u32 {
    u32::try_from(text.lines().filter(|line| !line.is_empty()).count()).unwrap_or(u32::MAX)
}

fn git_output(repo: &Path, trust_repository: bool, args: &[&str]) -> Result<String, String> {
    let mut command = crate::noninteractive_process::command("git");
    if trust_repository {
        command
            .arg("-c")
            .arg(format!("safe.directory={}", repo.display()));
    }
    let output = command
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first_line = stderr.lines().next().unwrap_or("").trim();
        return Err(if first_line.is_empty() {
            format!("git {} failed with status {}", args[0], output.status)
        } else {
            first_line.to_string()
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "herdr-removal-check-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn git(repo: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "git -C {} {}",
            repo.display(),
            args.join(" ")
        );
    }

    fn init_repo(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        git(path, &["init", "--quiet"]);
        git(path, &["config", "user.email", "herdr@example.invalid"]);
        git(path, &["config", "user.name", "MoMo Test"]);
    }

    fn commit(repo: &Path, file: &str) {
        std::fs::write(repo.join(file), file).unwrap();
        git(repo, &["add", file]);
        git(repo, &["commit", "--quiet", "-m", file]);
    }

    /// A clone of a bare remote whose single commit is pushed.
    fn pushed_clone(root: &Path, name: &str) -> PathBuf {
        let remote = root.join(format!("{name}-remote.git"));
        git(
            root,
            &["init", "--quiet", "--bare", remote.to_str().unwrap()],
        );
        let repo = root.join("checkout").join(name);
        init_repo(&repo);
        git(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        commit(&repo, "pushed.txt");
        git(
            &repo,
            &["push", "--quiet", "origin", "HEAD:refs/heads/main"],
        );
        git(&repo, &["fetch", "--quiet", "origin"]);
        repo
    }

    fn check(checkout: &Path) -> RemovalCheck {
        check_checkout_for_removal(checkout, false)
    }

    #[test]
    fn plain_directories_and_clean_pushed_repositories_are_safe() {
        let root = temp_dir("clean");
        let checkout = root.join("checkout");
        std::fs::create_dir_all(checkout.join("src/deep")).unwrap();
        std::fs::write(checkout.join("src/deep/file.txt"), "x").unwrap();
        pushed_clone(&root, "frontend");

        let result = check(&checkout);
        assert!(result.complete);
        assert!(result.at_risk.is_empty(), "{:?}", result.at_risk);
        assert!(result.refusal(&checkout).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn local_only_commits_and_unsaved_files_are_reported() {
        let root = temp_dir("unpushed");
        let checkout = root.join("checkout");
        let frontend = checkout.join("frontend");
        init_repo(&frontend);
        commit(&frontend, "local.txt");
        std::fs::write(frontend.join("unsaved.txt"), "unsaved").unwrap();

        let result = check(&checkout);
        assert_eq!(result.at_risk.len(), 1);
        let risk = &result.at_risk[0];
        assert_eq!(risk.relative_path, PathBuf::from("frontend"));
        assert_eq!(risk.kind, NestedRepositoryKind::Clone);
        assert_eq!(risk.unpushed_commits, 1);
        assert_eq!(risk.untracked, 1);
        assert_eq!(risk.summary(), "1 unpushed commit, 1 untracked path");

        let (refusal, message) = result.refusal(&checkout).unwrap();
        assert_eq!(refusal, RemovalRefusal::NestedRepositoriesAtRisk);
        assert!(
            message.contains("frontend: 1 unpushed commit, 1 untracked path"),
            "{message}"
        );
        assert!(message.contains("--discard-nested"), "{message}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn modified_files_and_stashes_in_pushed_repositories_are_reported() {
        let root = temp_dir("dirty");
        let checkout = root.join("checkout");
        let modified = pushed_clone(&root, "modified");
        std::fs::write(modified.join("pushed.txt"), "changed").unwrap();
        let stashed = pushed_clone(&root, "stashed");
        std::fs::write(stashed.join("pushed.txt"), "stash me").unwrap();
        git(&stashed, &["stash", "--quiet"]);

        let result = check(&checkout);
        let summaries = result
            .at_risk
            .iter()
            .map(|risk| (risk.relative_path.display().to_string(), risk.summary()))
            .collect::<Vec<_>>();
        assert_eq!(
            summaries,
            vec![
                ("modified".to_string(), "1 modified file".to_string()),
                ("stashed".to_string(), "1 stash".to_string()),
            ]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn repositories_nested_inside_nested_repositories_are_found() {
        let root = temp_dir("deep");
        let checkout = root.join("checkout");
        let outer = pushed_clone(&root, "outer");
        let inner = outer.join("libs").join("inner");
        init_repo(&inner);
        commit(&inner, "inner.txt");

        let result = check(&checkout);
        let paths = result
            .at_risk
            .iter()
            .map(|risk| risk.relative_path.clone())
            .collect::<Vec<_>>();
        // The outer repository sees `libs/` as untracked, so both are at risk.
        assert_eq!(
            paths,
            vec![PathBuf::from("outer"), PathBuf::from("outer/libs/inner")]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn worktrees_of_outside_repositories_only_risk_working_tree_changes() {
        let root = temp_dir("linked");
        let checkout = root.join("checkout");
        std::fs::create_dir_all(&checkout).unwrap();
        let outside = root.join("outside");
        init_repo(&outside);
        commit(&outside, "base.txt");
        let linked = checkout.join("linked");
        git(
            &outside,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                "task",
                linked.to_str().unwrap(),
            ],
        );
        // Committed-only work lives in `outside`, so the linked worktree is safe to delete.
        commit(&linked, "committed.txt");
        assert!(check(&checkout).at_risk.is_empty());

        std::fs::write(linked.join("base.txt"), "edited").unwrap();
        let result = check(&checkout);
        assert_eq!(result.at_risk.len(), 1);
        assert_eq!(result.at_risk[0].kind, NestedRepositoryKind::LinkedWorktree);
        assert_eq!(result.at_risk[0].summary(), "1 modified file");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn skipped_directories_are_not_searched() {
        let root = temp_dir("skipped");
        let checkout = root.join("checkout");
        let dependency = checkout.join("node_modules").join("pkg");
        init_repo(&dependency);
        commit(&dependency, "index.js");
        assert!(check(&checkout).at_risk.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        let root = temp_dir("symlink");
        let checkout = root.join("checkout");
        std::fs::create_dir_all(&checkout).unwrap();
        let outside = root.join("outside");
        init_repo(&outside);
        commit(&outside, "outside.txt");
        std::os::unix::fs::symlink(&outside, checkout.join("link")).unwrap();
        std::os::unix::fs::symlink(&checkout, checkout.join("loop")).unwrap();

        let result = check(&checkout);
        assert!(result.complete);
        assert!(result.at_risk.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn exhausted_budgets_mark_the_check_incomplete_and_refuse() {
        let root = temp_dir("budget");
        let checkout = root.join("checkout");
        std::fs::create_dir_all(checkout.join("a/b/c")).unwrap();
        for index in 0..5 {
            std::fs::write(checkout.join(format!("file-{index}")), "x").unwrap();
        }

        // a/b/c is three levels deep: a limit of three covers it exactly, two does not.
        assert!(check_with_budget(&checkout, false, 3, MAX_SCAN_ENTRIES).complete);
        let shallow = check_with_budget(&checkout, false, 2, MAX_SCAN_ENTRIES);
        assert!(!shallow.complete);
        let small = check_with_budget(&checkout, false, MAX_SCAN_DEPTH, 3);
        assert!(!small.complete);
        let (refusal, message) = small.refusal(&checkout).unwrap();
        assert_eq!(refusal, RemovalRefusal::CheckIncomplete);
        assert!(message.contains("--discard-nested"), "{message}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn acknowledgement_must_cover_every_repository_at_risk() {
        let root = temp_dir("ack");
        let checkout = root.join("checkout");
        for name in ["one", "two"] {
            let repo = checkout.join(name);
            init_repo(&repo);
            commit(&repo, "local.txt");
        }
        let result = check(&checkout);
        assert_eq!(result.at_risk.len(), 2);

        let all = vec![checkout.join("one"), checkout.join("two")];
        assert!(result.unacknowledged_refusal(&checkout, &all).is_none());
        let (_, message) = result
            .unacknowledged_refusal(&checkout, &all[..1])
            .expect("unreviewed repository is refused");
        assert!(message.contains("two:"), "{message}");
        assert!(!message.contains("one:"), "{message}");
        let _ = std::fs::remove_dir_all(root);
    }
}
