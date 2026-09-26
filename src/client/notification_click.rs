//! Clicking a system notification opens the pane that raised it.
//!
//! Each attached client listens on a private local socket. A clickable system
//! notification carries a shell command (run by `terminal-notifier -execute`)
//! that calls `herdr notification open-target`, which sends the notification's
//! opaque token back to that client; the client then navigates like the
//! existing "open notification target" action.
//!
//! Notification titles and bodies come from agents, so they are never placed in
//! the click command: it only contains the herdr executable, the socket path,
//! and a token this module generated, each single-quoted for `/bin/sh`.

use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use super::ClientLoopEvent;

/// Longest request line accepted from the control socket.
const MAX_REQUEST_BYTES: u64 = 512;
const MAX_TOKEN_LEN: usize = 64;

#[derive(serde::Serialize, serde::Deserialize)]
struct OpenNotificationRequest {
    open_notification: String,
}

/// Quote `value` as one `/bin/sh` word: single quotes, with embedded single
/// quotes closed, escaped, and reopened.
pub(super) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Tokens are generated here, but anything read back from the socket is checked
/// before use: short, ASCII alphanumeric plus `-`.
pub(super) fn valid_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_TOKEN_LEN
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub(super) fn click_command(herdr_exe: &Path, socket: &Path, token: &str) -> String {
    format!(
        "{} notification open-target --client-socket {} --token {}",
        shell_quote(&herdr_exe.to_string_lossy()),
        shell_quote(&socket.to_string_lossy()),
        shell_quote(token)
    )
}

/// Token from one request line, or `None` for anything malformed.
pub(super) fn parse_request(line: &str) -> Option<String> {
    let request: OpenNotificationRequest = serde_json::from_str(line.trim()).ok()?;
    valid_token(&request.open_notification).then_some(request.open_notification)
}

/// Ask the client listening on `socket` to open the notification `token`.
pub(crate) fn send_open_request(socket: &Path, token: &str) -> io::Result<()> {
    if !valid_token(token) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid notification token",
        ));
    }
    let mut stream = crate::ipc::connect_local_stream(socket)?;
    let request = serde_json::to_string(&OpenNotificationRequest {
        open_notification: token.to_owned(),
    })
    .map_err(io::Error::other)?;
    stream.write_all(request.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

pub(super) fn control_socket_path(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("{pid}.sock"))
}

/// A running control socket; removes its socket file when dropped.
pub(super) struct NotificationClickListener {
    socket: PathBuf,
    herdr_exe: PathBuf,
}

impl NotificationClickListener {
    /// Listen for notification clicks and forward them to the client loop.
    pub(super) fn start(
        dir: &Path,
        event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    ) -> io::Result<Self> {
        let herdr_exe = std::env::current_exe()?;
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
        sweep_stale_sockets(dir);
        let socket = control_socket_path(dir, std::process::id());
        crate::ipc::prepare_socket_path(&socket, |path| {
            format!("notification control socket {} is in use", path.display())
        })?;
        let listener = crate::ipc::bind_local_listener(&socket)?;
        crate::ipc::restrict_socket_permissions(&socket, 0o600)?;
        std::thread::Builder::new()
            .name("herdr-notification-clicks".into())
            .spawn(move || accept_clicks(&listener, &event_tx))?;
        Ok(Self { socket, herdr_exe })
    }

    pub(super) fn click_command(&self, token: &str) -> String {
        click_command(&self.herdr_exe, &self.socket, token)
    }
}

impl Drop for NotificationClickListener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Remove sockets left by clients that were killed before they could clean
/// up. A live client answers the connect, so its socket is kept.
fn sweep_stale_sockets(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "sock")
        {
            let _ = crate::ipc::prepare_socket_path(&path, |_| String::new());
        }
    }
}

fn accept_clicks(
    listener: &crate::ipc::LocalListener,
    event_tx: &tokio::sync::mpsc::Sender<ClientLoopEvent>,
) {
    use interprocess::local_socket::traits::ListenerExt as _;

    for connection in listener.incoming() {
        let stream = match connection {
            Ok(stream) => stream,
            Err(err) => {
                debug!(err = %err, "notification click connection failed");
                continue;
            }
        };
        let mut line = String::new();
        let read =
            BufReader::new(std::io::Read::take(stream, MAX_REQUEST_BYTES)).read_line(&mut line);
        if let Err(err) = read {
            debug!(err = %err, "notification click read failed");
            continue;
        }
        if line.trim().is_empty() {
            // Another client's stale-socket sweep probing that this one is alive.
            continue;
        }
        let Some(token) = parse_request(&line) else {
            warn!("ignored malformed notification click request");
            continue;
        };
        if event_tx
            .blocking_send(ClientLoopEvent::NotificationClicked { token })
            .is_err()
        {
            // The client loop has exited.
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn sh_echo(word: &str) -> String {
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("printf '%s' {word}"))
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn shell_quoting_survives_hostile_values() {
        for value in [
            "/Users/me/Library/Application Support/herdr/1.sock",
            "it's",
            "$(touch /tmp/herdr-pwned)",
            "`id`",
            "a;b && c | d",
            "\"double\" 'single' \\back",
            "",
        ] {
            assert_eq!(sh_echo(&shell_quote(value)), value, "{value}");
        }
    }

    #[test]
    fn click_command_contains_only_quoted_paths_and_the_token() {
        let command = click_command(
            Path::new("/opt/herdr it's/herdr"),
            Path::new("/Users/me/.config/herdr/client-control/42.sock"),
            "n-7",
        );
        assert_eq!(
            command,
            r"'/opt/herdr it'\''s/herdr' notification open-target --client-socket '/Users/me/.config/herdr/client-control/42.sock' --token 'n-7'"
        );
    }

    #[test]
    fn tokens_and_requests_are_validated() {
        assert!(valid_token("42-7"));
        for token in ["", "a b", "x;y", "../z", &"a".repeat(MAX_TOKEN_LEN + 1)] {
            assert!(!valid_token(token), "{token}");
        }
        assert_eq!(
            parse_request("{\"open_notification\":\"42-7\"}\n").as_deref(),
            Some("42-7")
        );
        for line in [
            "",
            "garbage",
            "{\"open_notification\":\"bad token\"}",
            "{\"open_notification\":7}",
            "{\"other\":\"42-7\"}",
        ] {
            assert!(parse_request(line).is_none(), "{line}");
        }
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn listener_forwards_clicks_and_removes_its_socket() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "herdr-click-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(4);
        let listener = NotificationClickListener::start(&dir, event_tx).unwrap();
        let socket = control_socket_path(&dir, std::process::id());
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);

        // Garbage is ignored; a valid request reaches the loop.
        let mut stream = crate::ipc::connect_local_stream(&socket).unwrap();
        stream.write_all(b"not json\n").unwrap();
        drop(stream);
        send_open_request(&socket, "42-7").unwrap();
        let event = tokio::time::timeout(std::time::Duration::from_secs(5), event_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event, ClientLoopEvent::NotificationClicked { token } if token == "42-7"));
        assert!(send_open_request(&socket, "bad token").is_err());

        drop(listener);
        assert!(!socket.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn starting_sweeps_sockets_of_dead_clients_but_keeps_live_ones() {
        let dir = std::env::temp_dir().join(format!(
            "herdr-click-sweep-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // A killed client: its listener is gone but the socket file remains.
        let dead = dir.join("1.sock");
        drop(crate::ipc::bind_local_listener(&dead).unwrap());
        assert!(dead.exists());
        // A live client in another process: still accepting connections.
        let live = dir.join("2.sock");
        let live_listener = crate::ipc::bind_local_listener(&live).unwrap();

        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(4);
        let listener = NotificationClickListener::start(&dir, event_tx).unwrap();
        assert!(!dead.exists(), "stale socket must be removed");
        assert!(live.exists(), "a live client's socket must be kept");

        drop(listener);
        drop(live_listener);
        let _ = std::fs::remove_dir_all(dir);
    }
}
