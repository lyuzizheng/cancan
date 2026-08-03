#![cfg_attr(
    test,
    allow(
        dead_code,
        reason = "the dedicated connector route has no renderer command until the later onboarding checkpoint"
    )
)]

use super::*;
use std::{
    io::Read,
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    thread,
    time::{Duration as StdDuration, Instant as StdInstant},
};

const GMAIL_AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(180);
const GMAIL_CONNECTOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const GMAIL_CONNECTOR_MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GmailConnectorAuthorizeCommand<'a> {
    client_id: &'a str,
    loopback_port: u16,
    request_id: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GmailConnectorHostResponse<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_url: Option<&'a str>,
    request_id: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum GmailConnectorMessage {
    Ready {
        #[serde(rename = "protocolVersion")]
        protocol_version: u8,
        runtime: String,
        #[serde(rename = "environmentCleared")]
        environment_cleared: bool,
    },
    OpenExternalUrl {
        #[serde(rename = "authorizationUrl")]
        authorization_url: Zeroizing<String>,
        #[serde(rename = "requestId")]
        request_id: String,
    },
    WaitForCallback {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    CloseListener {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    Persist {
        #[serde(rename = "mailboxAddress")]
        mailbox_address: String,
        #[serde(rename = "refreshToken")]
        refresh_token: Zeroizing<String>,
        #[serde(rename = "requestId")]
        request_id: String,
    },
    Result {
        #[serde(rename = "mailboxAddress")]
        mailbox_address: String,
        #[serde(rename = "requestId")]
        request_id: String,
    },
    Error {
        code: String,
    },
}

struct GmailLoopbackListener {
    listener: Option<TcpListener>,
    port: u16,
}

impl GmailLoopbackListener {
    fn bind() -> Result<Self, RuntimeError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| RuntimeError::new("gmail_authorization_unavailable"))?;
        listener
            .set_nonblocking(true)
            .map_err(|_| RuntimeError::new("gmail_authorization_unavailable"))?;
        let port = listener
            .local_addr()
            .map_err(|_| RuntimeError::new("gmail_authorization_unavailable"))?
            .port();
        Ok(Self {
            listener: Some(listener),
            port,
        })
    }

    fn close(&mut self) {
        self.listener.take();
    }

    fn take(&mut self) -> Result<TcpListener, RuntimeError> {
        self.listener
            .take()
            .ok_or_else(|| RuntimeError::new("gmail_authorization_failed"))
    }
}

pub(super) async fn run_gmail_connector_sidecar(
    app: &AppHandle,
    runtime: &VaultRuntime,
    client_id: &str,
) -> Result<String, RuntimeError> {
    let mut loopback = GmailLoopbackListener::bind()?;
    let request_id = random_identifier("gmail-oauth");
    let sidecar = app
        .shell()
        .sidecar("cancan-gmail-connector")
        .map_err(|_| RuntimeError::new("gmail_authorization_unavailable"))?
        .env_clear();
    let (mut events, mut child) = sidecar
        .spawn()
        .map_err(|_| RuntimeError::new("gmail_authorization_unavailable"))?;
    let deadline = Instant::now() + GMAIL_AUTHORIZATION_TIMEOUT;

    let protocol_result: Result<String, RuntimeError> = async {
        let ready = next_gmail_connector_message(&mut events, deadline).await?;
        if !matches!(
            ready,
            GmailConnectorMessage::Ready {
                protocol_version: 1,
                ref runtime,
                environment_cleared: true,
            } if runtime == "gmail-oauth-v1"
        ) {
            return Err(RuntimeError::new("gmail_authorization_unavailable"));
        }
        write_gmail_connector_command(
            &mut child,
            &GmailConnectorAuthorizeCommand {
                client_id,
                loopback_port: loopback.port,
                request_id: &request_id,
                kind: "authorize",
            },
        )?;

        expect_open_external_url(app, &mut events, &mut child, deadline, &request_id).await?;
        let wait = next_gmail_connector_message(&mut events, deadline).await?;
        if !matches!(
            wait,
            GmailConnectorMessage::WaitForCallback {
                request_id: ref response_id
            } if response_id == &request_id
        ) {
            return Err(RuntimeError::new("gmail_authorization_failed"));
        }
        let listener = loopback.take()?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let callback_url = tauri::async_runtime::spawn_blocking(move || {
            wait_for_loopback_callback(listener, remaining)
        })
        .await
        .map_err(|_| RuntimeError::new("gmail_authorization_failed"))??;
        write_gmail_connector_command(
            &mut child,
            &GmailConnectorHostResponse {
                callback_url: Some(&callback_url),
                request_id: &request_id,
                kind: "callback",
            },
        )?;

        let close = next_gmail_connector_message(&mut events, deadline).await?;
        if !matches!(
            close,
            GmailConnectorMessage::CloseListener {
                request_id: ref response_id
            } if response_id == &request_id
        ) {
            return Err(RuntimeError::new("gmail_authorization_failed"));
        }
        loopback.close();
        write_gmail_connector_response(&mut child, &request_id, "closed")?;

        let persist = next_gmail_connector_message(&mut events, deadline).await?;
        let persisted_mailbox = match persist {
            GmailConnectorMessage::Persist {
                mailbox_address,
                refresh_token,
                request_id: response_id,
            } if response_id == request_id => {
                runtime.persist_gmail_mailbox(&mailbox_address, refresh_token.as_bytes())?;
                mailbox_address
            }
            GmailConnectorMessage::Error { code } => {
                return Err(RuntimeError::new(gmail_connector_error_code(&code)));
            }
            _ => return Err(RuntimeError::new("gmail_authorization_failed")),
        };
        Ok(persisted_mailbox)
    }
    .await;
    let persisted_mailbox = match protocol_result {
        Ok(mailbox) => mailbox,
        Err(error) => {
            let _ = child.kill();
            return Err(error);
        }
    };

    // Vault and Keychain persistence is the commit point. Protocol completion
    // and process shutdown remain best-effort cleanup from here onward.
    if write_gmail_connector_response(&mut child, &request_id, "persisted").is_err() {
        let _ = child.kill();
        return Ok(persisted_mailbox);
    }
    let result_matches = next_gmail_connector_message(&mut events, deadline)
        .await
        .is_ok_and(|message| {
            matching_gmail_connector_result(&message, &request_id, &persisted_mailbox)
        });
    if !result_matches {
        let _ = child.kill();
        return Ok(persisted_mailbox);
    }
    if write_gmail_connector_shutdown(&mut child).is_err() {
        let _ = child.kill();
        return Ok(persisted_mailbox);
    }
    let shutdown_deadline = Instant::now() + GMAIL_CONNECTOR_SHUTDOWN_TIMEOUT;
    if !matches!(
        timeout_at(shutdown_deadline, events.recv()).await,
        Ok(Some(CommandEvent::Terminated(payload))) if payload.code == Some(0)
    ) {
        let _ = child.kill();
    }
    Ok(persisted_mailbox)
}

async fn expect_open_external_url(
    app: &AppHandle,
    events: &mut tokio::sync::mpsc::Receiver<CommandEvent>,
    child: &mut CommandChild,
    deadline: Instant,
    request_id: &str,
) -> Result<(), RuntimeError> {
    let message = next_gmail_connector_message(events, deadline).await?;
    let GmailConnectorMessage::OpenExternalUrl {
        authorization_url,
        request_id: response_id,
    } = message
    else {
        return Err(RuntimeError::new("gmail_authorization_failed"));
    };
    if response_id != request_id || !valid_google_authorization_url(&authorization_url) {
        return Err(RuntimeError::new("gmail_authorization_failed"));
    }
    #[allow(
        deprecated,
        reason = "the existing Rust-only shell plugin remains the narrow browser opener until the desktop shell adopts tauri-plugin-opener"
    )]
    app.shell()
        .open(authorization_url.as_str(), None)
        .map_err(|_| RuntimeError::new("gmail_authorization_failed"))?;
    write_gmail_connector_response(child, request_id, "opened")
}

async fn next_gmail_connector_message(
    events: &mut tokio::sync::mpsc::Receiver<CommandEvent>,
    deadline: Instant,
) -> Result<GmailConnectorMessage, RuntimeError> {
    let event = match timeout_at(deadline, events.recv()).await {
        Ok(Some(event)) => event,
        Ok(None) | Err(_) => return Err(RuntimeError::new("gmail_authorization_failed")),
    };
    match event {
        CommandEvent::Stdout(bytes) if bytes.len() <= GMAIL_CONNECTOR_MAX_MESSAGE_BYTES => {
            let bytes = Zeroizing::new(bytes);
            serde_json::from_slice(&bytes)
                .map_err(|_| RuntimeError::new("gmail_authorization_failed"))
        }
        CommandEvent::Terminated(_)
        | CommandEvent::Stderr(_)
        | CommandEvent::Error(_)
        | CommandEvent::Stdout(_) => Err(RuntimeError::new("gmail_authorization_failed")),
        _ => Err(RuntimeError::new("gmail_authorization_failed")),
    }
}

fn write_gmail_connector_command<T: Serialize>(
    child: &mut CommandChild,
    command: &T,
) -> Result<(), RuntimeError> {
    let mut bytes = Zeroizing::new(
        serde_json::to_vec(command).map_err(|_| RuntimeError::new("gmail_authorization_failed"))?,
    );
    bytes.push(b'\n');
    child
        .write(&bytes)
        .map_err(|_| RuntimeError::new("gmail_authorization_failed"))
}

fn write_gmail_connector_response(
    child: &mut CommandChild,
    request_id: &str,
    kind: &'static str,
) -> Result<(), RuntimeError> {
    write_gmail_connector_command(
        child,
        &GmailConnectorHostResponse {
            callback_url: None,
            request_id,
            kind,
        },
    )
}

fn write_gmail_connector_shutdown(child: &mut CommandChild) -> Result<(), RuntimeError> {
    child
        .write(b"{\"type\":\"shutdown\"}\n")
        .map_err(|_| RuntimeError::new("gmail_authorization_failed"))
}

fn gmail_connector_error_code(code: &str) -> &'static str {
    match code {
        "gmail_request_failed" => "gmail_request_failed",
        "gmail_authorization_failed" => "gmail_authorization_failed",
        _ => "gmail_authorization_failed",
    }
}

fn matching_gmail_connector_result(
    message: &GmailConnectorMessage,
    request_id: &str,
    persisted_mailbox: &str,
) -> bool {
    matches!(
        message,
        GmailConnectorMessage::Result {
            mailbox_address,
            request_id: response_id,
        } if response_id == request_id && mailbox_address == persisted_mailbox
    )
}

fn valid_google_authorization_url(url: &str) -> bool {
    url.len() <= 16 * 1024
        && url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?")
        && !url.contains(['\r', '\n'])
}

fn wait_for_loopback_callback(
    listener: TcpListener,
    timeout: StdDuration,
) -> Result<Zeroizing<String>, RuntimeError> {
    let deadline = StdInstant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((mut stream, peer)) if peer.ip().is_loopback() => {
                let address = listener
                    .local_addr()
                    .map_err(|_| RuntimeError::new("gmail_authorization_failed"))?;
                return read_loopback_request(&mut stream, address);
            }
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if StdInstant::now() >= deadline {
                    return Err(RuntimeError::new("gmail_authorization_failed"));
                }
                thread::sleep(StdDuration::from_millis(10));
            }
            Err(_) => return Err(RuntimeError::new("gmail_authorization_failed")),
        }
    }
}

fn read_loopback_request(
    stream: &mut TcpStream,
    listener_address: SocketAddr,
) -> Result<Zeroizing<String>, RuntimeError> {
    stream
        .set_read_timeout(Some(StdDuration::from_secs(10)))
        .map_err(|_| RuntimeError::new("gmail_authorization_failed"))?;
    let mut request = Zeroizing::new(Vec::with_capacity(1024));
    let mut chunk = Zeroizing::new([0_u8; 512]);
    loop {
        let read = stream
            .read(&mut chunk[..])
            .map_err(|_| RuntimeError::new("gmail_authorization_failed"))?;
        if read == 0 || request.len() + read > 8192 {
            return Err(RuntimeError::new("gmail_authorization_failed"));
        }
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let request_line = request
        .split(|byte| *byte == b'\n')
        .next()
        .and_then(|line| std::str::from_utf8(line).ok())
        .map(str::trim_end)
        .ok_or_else(|| RuntimeError::new("gmail_authorization_failed"))?;
    let mut parts = request_line.split(' ');
    let (Some("GET"), Some(target), Some("HTTP/1.1"), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(RuntimeError::new("gmail_authorization_failed"));
    };
    if !target.starts_with('/') || target.contains(['\r', '\n']) {
        return Err(RuntimeError::new("gmail_authorization_failed"));
    }
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 70\r\nConnection: close\r\n\r\n<!doctype html><title>CanCan</title>You may close this browser window.",
        )
        .map_err(|_| RuntimeError::new("gmail_authorization_failed"))?;
    Ok(Zeroizing::new(format!(
        "http://127.0.0.1:{}{target}",
        listener_address.port()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    #[test]
    fn binds_a_random_root_loopback_and_returns_one_bounded_callback() {
        let mut loopback = GmailLoopbackListener::bind().expect("bind loopback");
        let port = loopback.port;
        let listener = loopback.take().expect("take listener");
        // Keep the window bounded but generous enough that parallel test
        // scheduling on constrained CI runners cannot starve the callback
        // thread into a false timeout.
        let callback =
            thread::spawn(move || wait_for_loopback_callback(listener, StdDuration::from_secs(10)));
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connect loopback");
        stream
            .set_read_timeout(Some(StdDuration::from_secs(10)))
            .expect("set loopback client read timeout");
        stream
            .write_all(b"GET /?code=synthetic-code&state=synthetic-state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .expect("write callback");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read response");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert_eq!(
            callback
                .join()
                .expect("callback thread")
                .expect("valid callback")
                .as_str(),
            format!("http://127.0.0.1:{port}/?code=synthetic-code&state=synthetic-state")
        );
    }

    #[test]
    fn loopback_wait_is_bounded() {
        let mut loopback = GmailLoopbackListener::bind().expect("bind loopback");
        let started = StdInstant::now();
        let error = wait_for_loopback_callback(
            loopback.take().expect("take listener"),
            StdDuration::from_millis(20),
        )
        .expect_err("timeout");
        assert_eq!(error.code(), "gmail_authorization_failed");
        assert!(started.elapsed() < StdDuration::from_secs(1));
    }

    #[test]
    fn accepts_only_the_google_desktop_authorization_endpoint() {
        assert!(valid_google_authorization_url(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id=synthetic"
        ));
        assert!(!valid_google_authorization_url(
            "https://accounts.google.com.evil.example/o/oauth2/v2/auth?client_id=synthetic"
        ));
        assert!(!valid_google_authorization_url(
            "https://accounts.google.com/o/oauth2/v2/auth?\nclient_id=synthetic"
        ));
    }

    #[test]
    fn recognizes_only_the_committed_mailbox_result() {
        assert!(matching_gmail_connector_result(
            &GmailConnectorMessage::Result {
                mailbox_address: "owner@example.com".to_owned(),
                request_id: "request-1".to_owned(),
            },
            "request-1",
            "owner@example.com",
        ));
        assert!(!matching_gmail_connector_result(
            &GmailConnectorMessage::Error {
                code: "gmail_authorization_failed".to_owned(),
            },
            "request-1",
            "owner@example.com",
        ));
        assert!(!matching_gmail_connector_result(
            &GmailConnectorMessage::Result {
                mailbox_address: "other@example.com".to_owned(),
                request_id: "request-1".to_owned(),
            },
            "request-1",
            "owner@example.com",
        ));
    }
}
