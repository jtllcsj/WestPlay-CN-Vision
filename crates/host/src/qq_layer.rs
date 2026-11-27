//! The QQ middle layer (the `web` folder of L-1124's QQMusicApi, GPL-3.0), served on
//! `127.0.0.1:8080` for `qq_music.js`: a Python process of WestPlay's own, started when WestPlay
//! does and stopped when it ends.
//!
//! It has to be a process of its own. The player's page is a browser context: it can't set a
//! `Cookie` header (`forbidden header name`), and `y.qq.com`'s API is another origin, so the account's
//! credential could reach it no other way (qq_music.js says the same). The pool of accounts lives in
//! that process alone: nothing of them passes through WestPlay or the page it draws.
//!
//! The interpreter is the copy shipped beside it (`python/`, the embeddable distribution), so a
//! player needs no Python of their own.

use std::path::{Path, PathBuf};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU16, Ordering};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The port the layer serves on (`config.toml`'s `server.port`, and qq_music.js's `BASE`).
/// The port the layer answers on when it's free: 8787, clear of the 8080 that Steam's own browser
/// (`steamwebhelper`) holds on players' PCs (PR #5, @ManDik-Chan). The others are tried in turn when
/// something has it, and 8080 stays last for a layer an older WestPlay left running.
pub const PORT: u16 = 8787;

/// The port the layer is answering on this run (the one `start` chose: [`PORT`] unless something else
/// had it). QQ Music's hidden page asks it there (`qq_music.rs`), which starts after the layer.
static SERVING: AtomicU16 = AtomicU16::new(PORT);

/// The port the layer answers on this run ([`PORT`] until one was started or found).
pub fn serving_port() -> u16 {
    SERVING.load(Ordering::Acquire)
}

/// How long to wait for the layer to answer before saying it didn't start.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// The folder the layer was shipped in (`qq-music-layer`, beside WestPlay's other resources).
pub fn folder(resources: &Path) -> PathBuf {
    resources.join("qq-music-layer")
}

/// The layer, running: its process, where its folder is, and the port it answers on.
pub struct Layer {
    child: Child,
    folder: PathBuf,
    port: u16,
}

impl Layer {
    /// Starts the layer in `folder` (the shipped `qq-music-layer`), or `None` if it couldn't: its
    /// interpreter is gone, or the process didn't start.
    pub fn start(folder: &Path) -> Option<Self> {
        // Already ours answering (a WestPlay left running, or a player's own): its own is not needed.
        // A port that merely answers isn't enough — 8080 is a busy one, and Steam's own browser
        // (`steamwebhelper`) holds it on players' PCs: taking that for the layer left the app's page
        // asking Steam for its songs (owner 2026-11-17, a player whose QQ Music said "couldn't load").
        // Ours already answering on any of them (a WestPlay left running, or one of an older build on
        // 8080): its own is not needed, and the page is told where it is.
        if let Some(port) = PORTS.iter().copied().find(|&port| ours_is_serving(port)) {
            crate::log!("[qq layer] already serving on {port}");
            SERVING.store(port, Ordering::Release);
            return None;
        }
        let python = plain(folder.join("python").join("python.exe"));
        // WestPlay's own entry point first: it applies the two corrections the layer's compiled
        // sources need (run_westplay.py says which), then runs the layer's own `run.pyc`. Taken
        // when it's there, so a build without it still starts the layer as it is.
        let web = plain(folder.join("web"));
        let run = [
            web.join("run_westplay.py"),
            web.join("run.py"),
            web.join("run.pyc"),
        ]
        .into_iter()
        .find(|each| each.is_file());
        let Some(run) = run else {
            crate::log!("[qq layer] not shipped: no run.py or run.pyc in {}", web.display());
            return None;
        };
        // The port to answer on: 8080 unless something else has it (the layer takes
        // `QQMUSIC_SERVER_PORT`), then the next free one of its own.
        let (port, why) = free_port();
        if let Some(why) = why {
            crate::log!("[qq layer] {why}");
        }
        SERVING.store(port, Ordering::Release);
        let child = Command::new(&python)
            .arg(&run)
            .current_dir(&web)
            // The port the player's page asks too (SERVING), whatever the bundled config.toml says: an
            // older install's config still names 8080.
            .env("QQMUSIC_SERVER_HOST", "127.0.0.1")
            .env("QQMUSIC_SERVER_PORT", port.to_string())
            // The layer rates itself at 60 requests a minute per address (`[security]` in its config), and
            // the page's own screen asks for far more than that at once (a home feed, its rows' covers, a
            // collection's songs, the lyrics): answered 429, the app showed nothing at all. Our own calls
            // come from loopback and go no further than this PC, so they're exempt (PR #5, @ManDik-Chan:
            // 65 requests in a row all answered 200 with this).
            //
            // The whole section goes as JSON: its reader splits environment keys on underscores, so a
            // nested `QQMUSIC_SECURITY__RATE_LIMIT_EXEMPT_IPS` would be taken apart wrong. That section
            // carries more than the exemption (its CORS origins and credentials above all), so the
            // bundled `config.toml` is read first and only this one key is added — replacing the section
            // outright dropped CORS and every signed-in request came back 422 (owner 2026-11-19: the
            // app's own page couldn't load, while anonymous requests answered fine).
            .env("QQMUSIC_SECURITY", security_with_exemption(&web))
            // Its own console is hidden: nothing of it shows in the cab.
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| crate::log!("[qq layer] couldn't start: {error}"))
            .ok()?;
        let mut layer = Self {
            child,
            folder: folder.to_owned(),
            port,
        };
        // Ends with WestPlay's own process, however it goes (see `end_with_parent`).
        let tied = layer.end_with_parent();
        if !layer.wait_ready() {
            crate::log!("[qq layer] no answer on {port} after {READY_TIMEOUT:?}");
            layer.stop();
            return None;
        }
        crate::log!(
            "[qq layer] serving on {} ({}), ends with WestPlay: {tied}",
            layer.port,
            folder.display()
        );
        Some(layer)
    }

    /// Waits for the port to answer, up to [`READY_TIMEOUT`].
    fn wait_ready(&mut self) -> bool {
        let started = Instant::now();
        while started.elapsed() < READY_TIMEOUT {
            if ours_is_serving(self.port) {
                return true;
            }
            // The process died (a broken install): no point waiting longer.
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        false
    }

    /// Where the layer's folder is (its accounts and config are there).
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The port it answers on (the page asks it there: `qq_music.rs`).
    /// Whether it's still answering (see `netease_layer::Layer::is_serving` for why the host has
    /// to ask): its process is up, and its health route answers.
    pub fn is_serving(&mut self) -> bool {
        // Its process up (a layer killed leaves the port dead too, but a process that ended while
        // something else took its port must not read as ours), and its own health route answering.
        matches!(self.child.try_wait(), Ok(None)) && ours_is_serving(self.port)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Ends its process, and the one it made of its own (uvicorn keeps none: its worker is its own).
    /// Puts the layer's process in a job of its own that ends it when WestPlay's own process ends:
    /// a player closing the window with a task manager, or a crash, leaves nothing serving for ever
    /// (owner 2026-11-16: it answered after WestPlay was gone).
    fn end_with_parent(&self) -> bool {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                SetInformationJobObject,
            };
            // SAFETY: plain handles, closed by `CloseHandle` when this function's own goes.
            unsafe {
                let job = CreateJobObjectW(None, None);
                let Ok(job) = job else { return false };
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let set = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    std::ptr::addr_of!(info).cast(),
                    std::mem::size_of_val(&info) as u32,
                );
                if set.is_err() {
                    let _ = windows::Win32::Foundation::CloseHandle(job);
                    return false;
                }
                let process = windows::Win32::System::Threading::OpenProcess(
                    windows::Win32::System::Threading::PROCESS_SET_QUOTA
                        | windows::Win32::System::Threading::PROCESS_TERMINATE,
                    false,
                    self.child.id(),
                );
                let assigned = process
                    .and_then(|process| {
                        let result = AssignProcessToJobObject(job, process);
                        let _ = windows::Win32::Foundation::CloseHandle(process);
                        result
                    })
                    .is_ok();
                // The job's handle goes with this function: the layer's process then ends with
                // WestPlay's, the handle being the last one (`KILL_ON_JOB_CLOSE`).
                let _ = std::os::windows::io::AsRawHandle::as_raw_handle(&self.child);
                let _ = job;
                assigned
            }
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The same path without Windows' verbatim prefix (`\\?\`): Tauri's resource folder has it, and
/// `Command::new` on such a path fails to start the program (its own `is_file` answering nothing).
fn plain(path: PathBuf) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

/// Whether something answers on the layer's port now (a plain connect: the layer's own `/` says
/// nothing more).
/// The layer's `[security]` section from its own `config.toml`, with loopback added to
/// `rate_limit_exempt_ips` (its own calls: see `start`). Unreadable: just the exemption.
fn security_with_exemption(web: &std::path::Path) -> String {
    let mut section = std::fs::read_to_string(web.join("config.toml"))
        .ok()
        .and_then(|text| toml_section(&text, "security"))
        .unwrap_or_default();
    section.insert("rate_limit_exempt_ips".into(), serde_json::json!(["127.0.0.1", "::1"]));
    serde_json::Value::Object(section).to_string()
}

/// A `[name]` section of `text` as JSON values: `key = value` lines, the four kinds `config.toml` uses
/// (strings, numbers, booleans, and arrays of strings). `None` when there's no such section.
fn toml_section(text: &str, name: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    let header = format!("[{name}]");
    let mut section = serde_json::Map::new();
    let mut inside = false;
    let mut seen = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
            seen |= inside;
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        // A trailing `# comment` off a value that isn't quoted.
        let value = if value.starts_with('"') || value.starts_with('[') {
            value
        } else {
            value.split('#').next().unwrap_or(value).trim()
        };
        let parsed = serde_json::from_str::<serde_json::Value>(value).ok().or_else(|| {
            // The TOML its reader writes: `["a", "b"]` is JSON already; a lone string may hold an
            // apostrophe or a `#`, which JSON takes as it is.
            value
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .map(|inner| serde_json::Value::String(inner.to_owned()))
        });
        if let Some(parsed) = parsed {
            section.insert(key.to_owned(), parsed);
        }
    }
    seen.then_some(section)
}

/// The ports the layer may answer on, in order: 8080, the one WestPlay has always used, then the ones
/// it moves to when something else has that (Steam's own browser does, on players' PCs).
const PORTS: &[u16] = &[PORT, 18080, 18081, 18082, 18083, 8080];

/// The first of [`PORTS`] nothing is listening on, and why 8080 wasn't taken when it wasn't (`None`:
/// it was free).
fn free_port() -> (u16, Option<String>) {
    let holder = holder_of(PORT);
    for &port in PORTS {
        if !port_taken(port) {
            let why = if port == PORT {
                None
            } else {
                Some(format!(
                    "{PORT} is taken{}, the layer answers on {port} instead",
                    holder.as_deref().map(|name| format!(" by {name}")).unwrap_or_default()
                ))
            };
            return (port, why);
        }
    }
    // Every one of them taken: the last, and the layer says why it couldn't start.
    (
        *PORTS.last().unwrap_or(&PORT),
        Some(format!("every port of {PORTS:?} is taken")),
    )
}

/// What has the port, as `steamwebhelper.exe (41860)` (`None`: nothing, or it couldn't be named).
fn holder_of(port: u16) -> Option<String> {
    let output = Command::new("netstat").args(["-ano", "-p", "TCP"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let local = format!(":{port} ");
    let pid = text
        .lines()
        .find(|line| line.contains(&local) && line.contains("LISTENING"))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|pid| pid.parse::<u32>().ok())?;
    let name = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .and_then(|text| text.split(',').next().map(|name| name.trim_matches('"').to_owned()))
        .filter(|name| !name.is_empty())?;
    Some(format!("{name} ({pid})"))
}

/// Whether the layer itself answers on `port`: a port that merely connects isn't it (Steam's browser
/// holds 8080 on players' PCs).
fn ours_is_serving(port: u16) -> bool {
    let Some(body) = http_get(port, "/openapi.json") else {
        return false;
    };
    body.contains("QQMusic API")
}

/// A plain HTTP GET of `path` on `127.0.0.1:port`, its body (`None`: nothing answered).
fn http_get(port: u16, path: &str) -> Option<String> {
    use std::io::{Read, Write};
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(600)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut body = Vec::new();
    let mut chunk = [0u8; 4096];
    // The whole of it is not needed: the answer says what it is early on.
    while let Ok(read) = stream.read(&mut chunk) {
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > 64 * 1024 {
            break;
        }
    }
    Some(String::from_utf8_lossy(&body).to_string())
}

/// Whether `port` is unusable: something answers on it, or it cannot be bound at all.
///
/// Answering is not the whole story. A port can be held without answering — Windows lets a
/// process keep a socket with no listener on it, and some do: the OEM control panel's
/// `GCUService.exe` (a Mechrevo laptop's, 2026-11-24) sat on 8787 with a bound socket that
/// never answered, so `connect` timed out and the port read as free. The layer was then told
/// to answer on it, failed to bind (`errno 13`, permission denied — the socket was someone
/// else's), and gave up after twenty seconds with QQ Music showing nothing.
///
/// So a port is free only if it can actually be bound. `bind` alone would not do either: a
/// WestPlay already answering on it holds the socket, and binding that one fails too, which
/// is the same answer we want.
fn port_taken(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    if TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok() {
        return true;
    }
    // Nothing answers; it is still no good if it cannot be taken. Closed at once: this only
    // asks whether the bind would work.
    std::net::TcpListener::bind(address).is_err()
}

fn port_open() -> bool {
    port_taken(PORT)
}

/// Windows' flag for a process with no console window of its own.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Spawns a process without a console window of its own (`CREATE_NO_WINDOW`, Windows).
#[cfg(windows)]
trait NoWindow {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl NoWindow for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt;
        CommandExt::creation_flags(self, flags)
    }
}

#[cfg(not(windows))]
trait NoWindow {
    fn creation_flags(&mut self, _flags: u32) -> &mut Self;
}

#[cfg(not(windows))]
impl NoWindow for Command {
    fn creation_flags(&mut self, _flags: u32) -> &mut Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_without_a_layer_starts_nothing() {
        let missing = Path::new("no-such-folder");
        assert!(Layer::start(missing).is_none());
    }

    /// A port held without a listener is taken, not free.
    ///
    /// This is what went wrong with a Mechrevo laptop's OEM control panel on 8787 (2026-11-24):
    /// it kept a socket bound and never listened, so answering-and-nothing-answering both said
    /// "free" and the layer was told to answer there — which it could not, and QQ Music showed
    /// nothing. A listener that is held here, and one that is not, are the two cases to tell
    /// apart.
    #[test]
    fn a_port_held_without_a_listener_is_taken() {
        // A socket bound and never listened on: `connect` fails, `bind` fails too.
        let held = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a free port");
        let port = held.local_addr().expect("its address").port();
        // `held` is a listener, so it answers: taken either way. What matters is the other one.
        assert!(port_taken(port), "a listening port is taken");

        // Closed, and nothing else on it: free.
        drop(held);
        assert!(!port_taken(port), "a closed port is free");
    }
}
