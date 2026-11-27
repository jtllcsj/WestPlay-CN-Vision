//! NetEase Cloud Music's middle layer: a Python process of WestPlay's own, started when WestPlay does
//! and stopped when it ends.
//!
//! It has to be a process of its own, for the same reason `qq_layer` is one: the page WestPlay draws is a
//! browser context, which can't set a `Cookie` header (`forbidden header name`), and `music.163.com` is
//! another origin — so the account's cookie could reach the service no other way. The account lives in
//! that process alone; nothing of it passes through WestPlay or the page it draws.
//!
//! The interpreter is the copy shipped beside it (`python/`), so a player needs no Python of their own.
//! Unlike QQ's layer this one runs no framework: it is the standard library only, so the runtime staged
//! beside it is a fraction of a full install (see `tools/dev/stage_python.py`).

use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

/// The port the layer answers on when it's free: clear of the QQ layer's 8787, of 8080 which Steam's own
/// browser (`steamwebhelper`) holds on many players' PCs, and of the 8765 the phone UI takes.
pub const PORT: u16 = 8788;

/// The ports to try: a WestPlay left running, or a player's own tool, can't keep the app from starting.
pub const PORTS: &[u16] = &[PORT, PORT + 1, PORT + 2, PORT + 3, PORT + 4, PORT + 5, PORT + 6, PORT + 7];

/// The port the layer is answering on this run (the one `start` chose: [`PORT`] unless something else
/// had it). The page asks it there (`netease_music`).
static SERVING: AtomicU16 = AtomicU16::new(PORT);

/// The port the layer answers on this run ([`PORT`] until one was started or found).
pub fn serving_port() -> u16 {
    SERVING.load(Ordering::Acquire)
}

/// How long to wait for the layer to answer before saying it didn't start.
const READY_TIMEOUT: Duration = Duration::from_secs(25);

/// The folder the layer was shipped in (`netease-layer`, beside WestPlay's other resources).
pub fn folder(resources: &Path) -> PathBuf {
    resources.join("netease-layer")
}

/// The layer, running: its process, and the port it answers on.
pub struct Layer {
    /// `None` when this adopted one already answering (a WestPlay left running): there's no child of
    /// ours to end, so leaving it alone is right.
    child: Option<Child>,
    port: u16,
}

impl Layer {
    /// Starts the layer in `folder` (the shipped `netease-layer`), or `None` if it couldn't: its runtime
    /// is gone, or the process never answered.
    pub fn start(folder: &Path) -> Option<Self> {
        // Ours already answering (a WestPlay left running, or a player's own tool): its own isn't needed.
        // The health route is asked rather than the port merely being open, so a stranger on it isn't
        // mistaken for the layer.
        if let Some(port) = PORTS.iter().copied().find(|&port| ours_is_serving(port)) {
            crate::log!("[netease layer] already serving on {port}");
            SERVING.store(port, Ordering::Release);
            return Some(Self { child: None, port });
        }

        let python = interpreter(folder)?;
        // The layer's code ships inside the runtime's folder, which is what lets `-m netease_layer` find
        // it with no `PYTHONPATH`: so the folder to check is the code, beside the interpreter.
        let code = python.parent().map(|dir| dir.join("netease_layer")).unwrap_or_else(|| folder.join("netease_layer"));
        if !code.is_dir() {
            crate::log!("[netease layer] {} isn't there", code.display());
            return None;
        }

        for &port in PORTS {
            if ours_is_serving(port) {
                // Taken between the check above and here: skip it.
                continue;
            }
            match spawn(&python, folder, port) {
                Ok(mut child) => {
                    if wait_until_answering(port, READY_TIMEOUT) {
                        SERVING.store(port, Ordering::Release);
                        crate::log!("[netease layer] answering on {port}");
                        return Some(Self { child: Some(child), port });
                    }
                    // Started but silent: end it and try the next port.
                    let _ = child.kill();
                    let _ = child.wait();
                }
                Err(error) => crate::log!("[netease layer] couldn't start on {port}: {error}"),
            }
        }
        crate::log!("[netease layer] no port would do; the layer isn't running");
        None
    }

    /// The port this layer answers on.
    /// Whether it's still answering: the process is up, and its health route answers.
    ///
    /// The host has no other way to know: the layer is a separate process, and one killed by a
    /// cleaner, by Windows under memory pressure, or by its own error leaves the app asking a port
    /// that no longer answers — what a player sees as the app failing to load anything, with
    /// nothing in the log (they restart it by hand: a player's own `.bat` for exactly this,
    /// 2026-11-25).
    pub fn is_serving(&mut self) -> bool {
        match self.child.as_mut() {
            // Ours: a process that ended is not serving, whatever answers on its port.
            Some(child) => matches!(child.try_wait(), Ok(None)) && health(self.port).is_some(),
            // Adopted (a WestPlay already running): the port is all there is to go on.
            None => health(self.port).is_some(),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Stops the layer, and waits for it to go.
    pub fn stop(&mut self) {
        let Some(child) = self.child.as_mut() else { return };
        // Closing stdin is how it's told: its reader sees EOF and shuts the server down.
        drop(child.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => return,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The Python that runs the layer: the copy shipped beside it.
///
/// Not the system's: this layer's modules must be the ones shipped beside it, and a player's own
/// interpreter may be a different version with a different standard library. A missing runtime is a
/// broken install, said plainly, not something to paper over.
fn interpreter(folder: &Path) -> Option<PathBuf> {
    let shipped = folder.join("python").join(if cfg!(windows) { "python.exe" } else { "python3" });
    if shipped.is_file() {
        return Some(shipped);
    }
    crate::log!("[netease layer] no shipped interpreter under {}", folder.display());
    None
}

/// Starts the layer's process on `port`, its stdin a pipe (closing it is how the layer is told to go).
///
/// No console window: this runs beside a game, and a black box dropping onto the player's screen is not
/// something they asked for. `python.exe` is a console program, so redirecting its stdio alone still
/// gives it a console — `CREATE_NO_WINDOW` is what Windows needs.
fn spawn(python: &Path, folder: &Path, port: u16) -> std::io::Result<Child> {
    // The account is kept beside the app's own log, so it outlives a restart.
    let state = crate::log::dir()
        .and_then(|dir| dir.parent().map(|up| up.join("netease")))
        .unwrap_or_else(|| folder.to_path_buf());
    let _ = std::fs::create_dir_all(&state);
    let mut command = Command::new(python);
    command
        .current_dir(folder)
        .arg("-m")
        .arg("netease_layer")
        .arg("--port")
        .arg(port.to_string())
        .arg("--state")
        .arg(&state)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// `CREATE_NO_WINDOW`: the child gets no console of its own.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
}

/// Whether the layer (not merely something on the port) answers on `port`.
fn ours_is_serving(port: u16) -> bool {
    health(port).is_some()
}

/// The layer's `/api/health`, read for the two things the page's tab needs.
///
/// `None` when the layer can't be reached: it's just starting, or it has gone. Deliberately forgiving — a
/// layer that isn't up must not keep the host from drawing the rest of the cabin.
/// Hands the account's cookie to the layer (`POST /api/signin/cookie`): the sign-in window's
/// page is where it was signed in, and the layer is what plays from that account.
/// Tells the layer to forget the account (`POST /api/signout`): the cabin's app stops playing
/// as it, and the next start has no account.
/// A `POST` with no body, answered as text.
fn post_empty(port: u16, path: &str) -> Result<String, String> {
    use std::io::{Read, Write};

    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| error.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .map_err(|error| error.to_string())?;
    Ok(answer.split("\r\n\r\n").nth(1).unwrap_or("").to_owned())
}

pub fn post_signout(port: u16) -> Result<String, String> {
    post_empty(port, "/api/signout")
}

pub fn post_cookie(port: u16, cookie: &str) -> Result<String, String> {
    use std::io::{Read, Write};

    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| error.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let body = serde_json::json!({ "cookie": cookie }).to_string();
    let request = format!(
        "POST /api/signin/cookie HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .map_err(|error| error.to_string())?;
    Ok(answer.split("\r\n\r\n").nth(1).unwrap_or("").to_owned())
}

pub fn health(port: u16) -> Option<(bool, String)> {
    use std::io::{Read, Write};

    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(300)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(900)));
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    let body = answer.split("\r\n\r\n").nth(1)?;
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    // Ours answers these two: a stranger's page on that port won't.
    if !parsed.get("ok").is_some_and(|v| v.as_bool() == Some(true)) {
        return None;
    }
    let signed_in = parsed.get("signedIn").and_then(|v| v.as_bool()).unwrap_or(false);
    let nickname = parsed.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    Some((signed_in, nickname))
}

/// Waits for the layer to answer, up to `timeout`.
fn wait_until_answering(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if ours_is_serving(port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The port sits clear of everything that commonly holds one on a player's PC.
    #[test]
    fn the_port_is_clear_of_the_other_local_services() {
        assert_ne!(PORT, 8787, "the QQ layer's");
        assert_ne!(PORT, 8080, "steamwebhelper's");
        assert_ne!(PORT, 8765, "the phone UI's");
        assert_eq!(PORT, 8788);
    }

    /// The tries stay inside the range a client would use, and don't run into the ephemeral ports.
    #[test]
    fn the_ports_tried_stay_below_the_ephemeral_range() {
        assert!(PORTS.iter().all(|&p| p < 49_152), "the ephemeral range would take them");
        assert!(PORTS.windows(2).all(|w| w[0] < w[1]), "in order, so the first free one is used");
    }

    /// A folder without the shipped runtime names no interpreter: a broken install must not look like a
    /// working layer.
    #[test]
    fn a_folder_without_the_shipped_runtime_names_no_interpreter() {
        let folder = std::env::temp_dir().join("westplay-netease-missing");
        let _ = std::fs::remove_dir_all(&folder);
        let _ = std::fs::create_dir_all(&folder);
        assert_eq!(interpreter(&folder), None, "no python/, no interpreter");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
