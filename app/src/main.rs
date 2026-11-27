//! WestPlay app: the companion window (Tauri) plus the in-cabin UI host on its own thread.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod customize;
mod maps;
mod setup;
mod update;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use setup::SetupStore;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewWindow};
use westplay_host::maps::SetUpGame;
use westplay_host::settings::{Settings, SettingsFile, SharedSettings};
use westplay_host::{
    AccountAction, AccountRequests, BrowserGone, Config, Content, OpenRequest, SCREEN_HEIGHT, SCREEN_WIDTH, Status,
};

struct Host {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Requests from the companion window to open or close WestPlay on the truck's screen (the cabin page owns
/// that state, which it also changes itself: its sidebar's truck button, its WestPlay button, the interaction
/// key and W; the host reports it back in its status).
struct WestplayOpen(Arc<OpenRequest>);

/// The companion's sign-ins and sign-outs (YouTube, Spotify, Apple Music), for the host.
#[derive(Default)]
struct Quitting {
    /// Answered: the sheet's Quit was pressed, so the window closes as it would have.
    asked: AtomicBool,
    /// The sheet is up: the close pressed again over it has nothing more to ask.
    shown: AtomicBool,
    /// This ask got its sheet, so the fallback close (a page that can't answer) isn't due.
    answered: AtomicBool,
}

const ASK_FALLBACK: Duration = Duration::from_secs(3);

struct Accounts(Arc<AccountRequests>);

/// The player's settings: kept on disk, applied live by the host thread, changed from the companion window.
struct SettingsStore {
    file: SettingsFile,
    shared: Arc<SharedSettings>,
}

/// The companion window starts hidden and shows once its page has drawn its first screen
/// (`companion_ready`), so it never shows blank while the page loads; after this long it shows anyway.
const SHOW_ANYWAY: Duration = Duration::from_secs(8);

/// The tray menu's two items, kept so the page can relabel them in its own language
/// (`set_tray_words`, which 3.0's companion calls). `None` until the tray is built.
#[derive(Default)]
struct TrayWords(std::sync::Mutex<Option<TrayWordsInner>>);

#[derive(Clone)]
struct TrayWordsInner {
    open: tauri::menu::MenuItem<tauri::Wry>,
    quit: tauri::menu::MenuItem<tauri::Wry>,
}

/// The player sent the window to the tray: it stays hidden until the tray asks for it, whatever
/// the page says about being ready (a game starting makes the companion load its setup again, and
/// `companion_ready` would bring the window back to the front — owner 2026-11-14).
static HIDDEN_BY_USER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// When the companion's preview (the demo iframe, `cabin.html?demo&power`) last said it was
/// alive, in seconds since WestPlay started (`Clock::secs` of `std::time::Instant`), or 0 when
/// it never has.
///
/// A heartbeat, not a process list: Edge WebView2 158's `STATUS_BREAKPOINT` kills the page's
/// **renderer** while the browser process stays up, so watching processes for a page's death
/// never sees it (2026-11-24, the analysis that named this). A page whose renderer is gone
/// stops sending these, whatever the browser around it is doing.
///
/// Only the preview beats: the hidden players (Spotify, YouTube, NetEase) do not, so one of
/// them closing is not read as a crash — the reason watching renderer processes was dropped in
/// the first place (2026-11-23, the window reloading itself as a player closed).
static PREVIEW_BEAT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The preview in the companion's setup is alive: it beats about once a second
/// (`companion/src/companion/Preview.tsx`).
///
/// Recorded here because the page cannot be asked: with its renderer dead WebView2 answers
/// nothing, and the whole point is to tell that apart from a page still working.
#[tauri::command]
fn preview_alive() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    PREVIEW_BEAT.store(now, std::sync::atomic::Ordering::Release);
}

/// The companion's page has drawn its first screen: the window shows.
#[tauri::command]
fn companion_ready(window: WebviewWindow) {
    if HIDDEN_BY_USER.load(std::sync::atomic::Ordering::Acquire) {
        return;
    }
    if !window.is_visible().unwrap_or(false) {
        westplay_host::startup::mark("app: window shown");
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Whether the companion's window is the one in front. Windows' word: the page's own `document.hasFocus()` says
/// yes while the window comes up behind another.
#[tauri::command]
fn window_in_front(window: WebviewWindow) -> bool {
    window.hwnd().is_ok_and(|window| westplay_host::in_front(window.0 as isize))
}

/// 页面自绘标题栏的最小化按钮（3.0 的窗口没有 Windows 标题栏，见 `tauri.conf.json` 的
/// `decorations:false`）。
#[tauri::command]
fn window_minimise(window: WebviewWindow) {
    let _ = window.minimize();
}

/// 页面自绘标题栏的关闭按钮：和 Windows 的关闭一样，收进托盘（`on_window_event` 里的
/// `CloseRequested` 负责真正的隐藏），托盘菜单才是退出。
#[tauri::command]
fn window_close(window: WebviewWindow) {
    westplay_host::log!("[quit] the window's close button: hidden to the tray");
    let _ = window.close();
}

/// 托盘菜单的两项文字，由页面按当前语言送过来（3.0 的 `set_tray_words`）。
#[tauri::command]
fn set_tray_words(words: State<TrayWords>, open: String, quit: String) {
    let kept = words.0.lock().ok().and_then(|kept| kept.clone());
    if let Some(kept) = kept {
        let _ = kept.open.set_text(open);
        let _ = kept.quit.set_text(quit);
    }
}

/// 拖拽：页面顶部的空白处按下时调它，让 Windows 接管这一次拖动（无边框窗口没有标题栏
/// 可以拖）。
#[tauri::command]
fn window_drag(window: WebviewWindow) {
    let _ = window.start_dragging();
}

/// The companion's page held its room still, or set it going again: for a log to tell what the room cost.
#[tauri::command]
fn room_held(held: bool) {
    westplay_host::log!("companion: {}", if held { "behind, its room held still" } else { "in front, its room going" });
}

#[tauri::command]
fn set_westplay_open(open: bool, request: State<WestplayOpen>) {
    request.0.request(open);
}

/// Opens Google's sign-in for YouTube in a window on the desktop (the status then says when it's done).
#[tauri::command]
fn youtube_sign_in(accounts: State<Accounts>) {
    accounts.0.youtube.request(AccountAction::SignIn);
}

/// Signs YouTube out.
#[tauri::command]
fn youtube_sign_out(accounts: State<Accounts>) {
    accounts.0.youtube.request(AccountAction::SignOut);
}

/// Runs YouTube (hidden), so the status says whether it's signed in.
#[tauri::command]
fn youtube_check(accounts: State<Accounts>) {
    accounts.0.youtube.request(AccountAction::Check);
}

/// Opens Spotify's sign-in in a window on the desktop (the status then says when it's done).
#[tauri::command]
fn spotify_sign_in(accounts: State<Accounts>) {
    accounts.0.spotify.request(AccountAction::SignIn);
}

/// Signs Spotify out.
#[tauri::command]
fn spotify_sign_out(accounts: State<Accounts>) {
    accounts.0.spotify.request(AccountAction::SignOut);
}

/// Runs Spotify's web player (hidden), so the status says whether it's signed in.
#[tauri::command]
fn spotify_check(accounts: State<Accounts>) {
    accounts.0.spotify.request(AccountAction::Check);
}

/// Opens Apple's sign-in for Apple Music in a window on the desktop (the status then says when it's done).
/// Asks Bilibili's client whether its page is signed in. Its page says so as it loads, so
/// this only has to be sure the client is running; nothing opens on the desktop.
#[tauri::command]
fn bilibili_check(accounts: State<Accounts>) {
    accounts.0.bilibili.request(AccountAction::Check);
}

#[tauri::command]
fn netease_check(accounts: State<Accounts>) {
    accounts.0.netease.request(AccountAction::Check);
}

#[tauri::command]
fn qq_music_check(accounts: State<Accounts>) {
    accounts.0.qq_music.request(AccountAction::Check);
}

#[tauri::command]
fn bilibili_sign_in(accounts: State<Accounts>) {
    accounts.0.bilibili.request(AccountAction::SignIn);
}

#[tauri::command]
fn netease_sign_in(accounts: State<Accounts>) {
    accounts.0.netease.request(AccountAction::SignIn);
}

#[tauri::command]
fn qq_music_sign_in(accounts: State<Accounts>) {
    accounts.0.qq_music.request(AccountAction::SignIn);
}

#[tauri::command]
fn qq_music_sign_out(accounts: State<Accounts>) {
    accounts.0.qq_music.request(AccountAction::SignOut);
}

#[tauri::command]
fn netease_sign_out(accounts: State<Accounts>) {
    accounts.0.netease.request(AccountAction::SignOut);
}

#[tauri::command]
fn bilibili_sign_out(accounts: State<Accounts>) {
    accounts.0.bilibili.request(AccountAction::SignOut);
}

#[tauri::command]
fn apple_music_sign_in(accounts: State<Accounts>) {
    accounts.0.apple_music.request(AccountAction::SignIn);
}

/// Signs Apple Music out.
#[tauri::command]
fn apple_music_sign_out(accounts: State<Accounts>) {
    accounts.0.apple_music.request(AccountAction::SignOut);
}

/// Runs Apple Music's web player (hidden), so the status says whether it's signed in.
#[tauri::command]
fn apple_music_check(accounts: State<Accounts>) {
    accounts.0.apple_music.request(AccountAction::Check);
}

/// The maker's Kreosus page, where players support WestPlay's development.
const SUPPORT_PAGE: &str = "https://afdian.com/a/west2026";

/// Opens the support page (Support me) in the player's browser. The address is fixed here, so the companion
/// can't open any other.
#[tauri::command]
fn open_support_page() {
    westplay_host::browser::open(SUPPORT_PAGE);
}

/// WestPlay's pages elsewhere, at the foot of the companion's list.
#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Social {
    /// 我们的 B站主页。
    Bilibili,
    /// 我们的 GitHub 仓库（`github`）。小千建议把中间那格换成它（owner 2026-11-23）：
    /// 支持我们的项目，也让玩家有个地方报问题、看更新。
    Github,
    /// The Chinese players' own way of supporting WestPlay (「爱发电」, afdian.com)。
    Afdian,
}

/// Opens one of WestPlay's pages elsewhere (its YouTube channel, its Discord, its Patreon) in the player's browser.
/// The addresses are fixed here, as the support page's.
#[tauri::command]
fn open_social(site: Social) {
    westplay_host::browser::open(match site {
        // 主窗口左下角的三个按钮（owner 2026-11-23）。
        Social::Bilibili => "https://space.bilibili.com/391801787",
        Social::Github => "https://github.com/LucianLuu/WestPlay-CN-Vision",
        Social::Afdian => "https://afdian.com/a/west2026",
    });
}

/// Watches the companion's page and puts it back if its process dies.
///
/// The cab's pages are watched where they are made (`westplay_host::webview`'s `ProcessFailed`
/// handler). The companion's window is Tauri's, made from `tauri.conf.json`, and Tauri doesn't hand out
/// the `CoreWebView2` that handler needs — so the page is watched from outside instead: WebView2 runs
/// each page in an `msedgewebview2.exe` child of this process, and a page that dies (WebView2 158's
/// STATUS_BREAKPOINT at the Setup) leaves its child gone.
///
/// Not the window's own process id: that is WestPlay's, which never changes. The first version of this
/// watched exactly that and could never have noticed anything.
///
/// It navigates rather than evaluates `location.reload()`: by the time a page's process is gone Chromium
/// has put its own error page up ("This page is having a problem", STATUS_BREAKPOINT), and reloading
/// *that* is not putting the Setup back — the first version did exactly this and the player's window
/// stayed on the error page. A navigation from here goes through the host to WebView2 itself, whatever
/// the renderer is doing.
fn watch_companion(app: tauri::AppHandle) {
    use tauri::Manager;

    /// How long the preview may go without a beat before its page is taken for dead (seconds):
    /// it beats every second, so this is several misses, and longer than any hitch a slow frame
    /// or a paused window is worth (the window being hidden holds its animations, not its
    /// timers).
    const PREVIEW_SILENT: u64 = 6;

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let Some(window) = app.get_webview_window("main") else { return };
            if window.hwnd().is_err() {
                // The window gone is the app closing: nothing left to watch.
                return;
            }

            // **One signal: the preview's heartbeat.** Nothing else here.
            //
            // Watching WebView2 browser processes was tried and doesn't work: the list holds
            // *every* browser under the app, and each sign-in window and hidden player has one
            // of its own. Closing any of them took a browser out of the list, which read as the
            // companion's page dying — the window reloaded itself every time a player closed the
            // sign-in sheet they had just opened (owner 2026-11-24), and before that whenever a
            // player closed (2026-11-23, "the main window keeps refreshing").
            //
            // The heartbeat has neither problem. The preview lives *inside* the companion's page,
            // so that page dying stops it; a sign-in window or a player closing changes nothing
            // about it. It also catches the case processes cannot: Edge WebView2 158's
            // STATUS_BREAKPOINT kills the renderer and leaves the browser process up.
            //
            // Only once it has beaten at least once — a window still loading its Setup, or one
            // whose setup was never opened, has no preview to lose.
            let beat = PREVIEW_BEAT.load(Ordering::Acquire);
            if beat == 0 {
                continue;
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_secs())
                .unwrap_or(0);
            let silent = now.saturating_sub(beat);
            if silent <= PREVIEW_SILENT {
                continue;
            }
            // Put back to 0: the beat that comes with the reloaded page starts it again, and
            // until then a second reload of a page that is still loading is no help.
            PREVIEW_BEAT.store(0, Ordering::Release);

            {
                // Where the page should be: what it was showing, or the Setup's own address.
                // Where the Setup lives: on Windows Tauri serves the bundled page from
                // `http://tauri.localhost/` (its `wry` workaround for `tauri://localhost`). Falling
                // back to it matters — the address asked for is the one *before* the crash, and a
                // page already showing Chromium's error page may report that error page's address.
                let address = window
                    .url()
                    .ok()
                    .filter(|url| {
                        url.path().ends_with(".html")
                            || url.path().trim_end_matches('/').is_empty()
                    })
                    .unwrap_or_else(|| {
                        tauri::Url::parse("http://tauri.localhost/index.html")
                            .expect("the Setup's own address")
                    });
                westplay_host::log!(
                    "[companion] the preview stopped answering {silent}s ago; going back to {address}"
                );
                let _ = window.navigate(address);
            }
        }
    });
}

/// A line from the companion's page, in `app.log`, as `[probe] …`.
///
/// The page can't write the log itself, and the seconds before a page dies are exactly where its own
/// account is needed: the Setup's check used to be followed by a crash 3.4 s later with nothing in the
/// log in between (a player's, 2026-11-22).
#[tauri::command]
fn page_log(line: String) {
    westplay_host::log!("{line}");
}

#[tauri::command]
fn quit_shown(window: WebviewWindow) {
    westplay_host::log!("[quit] the sheet is up, asking whether WestPlay quits");
    let quitting = window.state::<Quitting>();
    quitting.shown.store(true, Ordering::Release);
    quitting.answered.store(true, Ordering::Release);
}

#[tauri::command]
fn quit_hidden(window: WebviewWindow) {
    westplay_host::log!("[quit] the sheet was closed without quitting");
    window.state::<Quitting>().shown.store(false, Ordering::Release);
}

#[tauri::command]
fn quit_app(app: AppHandle, window: WebviewWindow) {
    westplay_host::log!("[quit] the sheet says WestPlay quits");
    window.state::<Quitting>().asked.store(true, Ordering::Release);
    app.exit(0);
}

/// Where this run's `app.log` is (the About page): Documents\WestPlay\logs, or WestPlay's own data folder where
/// Documents refused it; `None` if it couldn't be made.
#[tauri::command]
fn app_log_place() -> Option<String> {
    westplay_host::log::place().map(|path| path.display().to_string())
}

/// A keyboard key's name for the player's keyboard, by its virtual-key code (the keybinds' labels); `None`
/// for a mouse button or a key without one.
#[tauri::command]
fn key_name(key: u32) -> Option<String> {
    westplay_host::settings::KeyCode::name(key)
}

#[tauri::command]
fn get_settings(store: State<SettingsStore>) -> Settings {
    store.shared.get()
}

/// Saves new settings and applies them. Values are corrected on the way in (see `westplay_host::settings`);
/// the settings as saved are returned and announced to every window (`settings-changed`).
#[tauri::command]
fn update_settings(settings: Settings, store: State<SettingsStore>, app: AppHandle) -> Result<Settings, String> {
    store.file.save(&settings).map_err(|e| format!("Couldn't save the settings: {e}"))?;
    store.shared.set(settings.clone());
    let _ = app.emit("settings-changed", &settings);
    Ok(settings)
}

/// Where WestPlay's shipped resources are (`ui/`, `qq-music-layer/`): beside the app in an install,
/// or the source folder in a development build.
fn resources_folder(app: &tauri::AppHandle) -> Option<PathBuf> {
    use tauri::Manager as _;
    let resources = app.path().resource_dir().ok()?;
    if resources.join("qq-music-layer").is_dir() {
        return Some(resources);
    }
    // A development build: beside the executable, then the checkout's own folder.
    let beside = std::env::current_exe().ok()?.parent()?.to_owned();
    if beside.join("qq-music-layer").is_dir() {
        return Some(beside);
    }
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.to_owned();
    source.join("qq-music-layer").is_dir().then_some(source)
}

/// The QQ middle layer, kept alive by the app (its `Drop` ends it when WestPlay does).
struct QqLayer(std::sync::Mutex<Option<westplay_host::qq_layer::Layer>>);

/// NetEase Cloud Music's middle layer, kept alive by the app the same way: a local would be dropped
/// at the end of the setup closure, ending the layer with it.
struct NeteaseLayer(std::sync::Mutex<Option<westplay_host::netease_layer::Layer>>);

/// Keeps the two middle layers serving: one that stops answering is started again.
///
/// They are separate processes. Nothing told the app when one ended — killed by a cleaner, by
/// Windows reclaiming memory, or by its own error — and the app went on asking a port that no
/// longer answered: what a player saw as "网易云音乐/QQ 音乐 无法载入", with nothing in the log.
/// A player wrote a `.bat` that kills the leftover and starts it again by hand (2026-11-25);
/// this is the app doing that itself.
///
/// Only a layer the app started is restarted (one adopted from a WestPlay already running isn't
/// ours to end, and its own owner watches it). A layer that fails to come back is left alone: a
/// broken install would otherwise be started again every minute for as long as the app runs.
fn watch_layers(app: tauri::AppHandle) {
    use tauri::Manager;
    use std::time::Duration;

    /// How often the two are asked, and how long a layer that failed is left alone before the
    /// next try (a launch that fails takes a while to fail; retrying at that pace forever is noise).
    const EVERY: Duration = Duration::from_secs(20);
    const GIVE_UP_AFTER: u32 = 5;

    std::thread::spawn(move || {
        let mut failures: u32 = 0;
        loop {
            std::thread::sleep(EVERY);
            let Some(resources) = resources_folder(&app) else { continue };

            // QQ Music's (`qq_layer.rs`).
            if let Some(state) = app.try_state::<QqLayer>() {
                let Ok(mut kept) = state.0.lock() else { continue };
                let dead = kept.as_mut().is_some_and(|layer| !layer.is_serving());
                if dead {
                    if failures >= GIVE_UP_AFTER {
                        continue;
                    }
                    failures += 1;
                    westplay_host::log!("[qq layer] stopped answering; starting it again");
                    *kept = westplay_host::qq_layer::Layer::start(
                        &westplay_host::qq_layer::folder(&resources),
                    );
                    westplay_host::log!(
                        "[qq layer] restarted: {}",
                        if kept.is_some() { "serving" } else { "no answer yet" }
                    );
                    if kept.is_some() {
                        failures = 0;
                    }
                }
            }

            // NetEase Cloud Music's (`netease_layer.rs`).
            if let Some(state) = app.try_state::<NeteaseLayer>() {
                let Ok(mut kept) = state.0.lock() else { continue };
                let dead = kept.as_mut().is_some_and(|layer| !layer.is_serving());
                if dead {
                    if failures >= GIVE_UP_AFTER {
                        continue;
                    }
                    failures += 1;
                    westplay_host::log!("[netease layer] stopped answering; starting it again");
                    *kept = westplay_host::netease_layer::Layer::start(
                        &westplay_host::netease_layer::folder(&resources),
                    );
                    westplay_host::log!(
                        "[netease layer] restarted: {}",
                        if kept.is_some() { "serving" } else { "no answer yet" }
                    );
                    if kept.is_some() {
                        failures = 0;
                    }
                }
            }
        }
    });
}

fn main() {
    // Started as administrator only to copy the plugin into the game (setup asked for it): that, and done.
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = westplay_setup::plugin::run_as_administrator_copy(&args) {
        std::process::exit(code);
    }
    // One WestPlay at a time: a second only says so (held until the app ends). Before the log starts: a second
    // making the log anew would empty the running one's.
    let Some(_instance) = westplay_host::Instance::claim() else {
        westplay_host::tell_already_running();
        return;
    };
    westplay_host::log::init();
    westplay_host::startup::mark("app: started");
    let open_request = Arc::new(OpenRequest::default());
    let accounts = Arc::new(AccountRequests::default());
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::Updates::default())
        .manage(TrayWords::default())
        .manage(WestplayOpen(open_request.clone()))
        .manage(Accounts(accounts.clone()))
        .manage(Quitting::default())
        .register_asynchronous_uri_scheme_protocol(maps::SCHEME, maps::serve)
        .register_asynchronous_uri_scheme_protocol(maps::CUSTOMIZE_SCHEME, maps::serve_customize)
        .invoke_handler(tauri::generate_handler![
            quit_hidden,
            quit_shown,
            quit_app,
            companion_ready,
            window_in_front,
            window_minimise,
            window_close,
            window_drag,
            set_tray_words,
            room_held,
            set_westplay_open,
            youtube_sign_in,
            youtube_sign_out,
            youtube_check,
            spotify_sign_in,
            spotify_sign_out,
            spotify_check,
            apple_music_sign_in,
            apple_music_sign_out,
            apple_music_check,
            open_support_page,
            open_social,
            app_log_place,
            page_log,
            preview_alive,
            customize::customize_list,
            customize::add_wallpapers,
            customize::add_icon_set,
            customize::remove_wallpaper,
            customize::remove_icon_set,
            update::update_offer,
            update::look_for_update,
            update::install_update,
            get_settings,
            update_settings,
            key_name,
            setup::setup_state,
            setup::game_running,
            setup::prepare_maps_again,
            setup::choose_game_folder,
            setup::check_game,
            setup::install_into_game,
            setup::finish_setup,
            setup::uninstall_from_game,
            setup::remove_data,
            setup::leave_pc,
            bilibili_check,
            netease_check,
            qq_music_check,
            bilibili_sign_in,
            netease_sign_in,
            qq_music_sign_in,
                    bilibili_sign_out,
                    netease_sign_out,
                    qq_music_sign_out,
        ])
        .on_window_event(|window, event| {
            // Windows 的缩放改了（换显示器、系统设置），或者窗口被系统改了大小：裁剪区域要按新的
            // 尺寸重算，否则旧的圆角区域还在，多出来的部分就被裁掉了。
            if let tauri::WindowEvent::ScaleFactorChanged { scale_factor, .. } = event {
                if let Ok(hwnd) = window.hwnd() {
                    westplay_host::window_corner::apply(hwnd.0 as isize, *scale_factor);
                }
            }
            if let tauri::WindowEvent::Resized(_) = event {
                if let Ok(hwnd) = window.hwnd() {
                    let scale = window.scale_factor().unwrap_or(1.0);
                    westplay_host::window_corner::apply(hwnd.0 as isize, scale);
                }
            }
            // The close button puts WestPlay in the tray instead of ending it: it keeps playing, and
            // the tray's menu is where it really quits.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                HIDDEN_BY_USER.store(true, std::sync::atomic::Ordering::Release);
                let _ = window.hide();
                westplay_host::log!("[quit] the window's close: hidden to the tray");
            }
        })
        .setup(move |app| {
            westplay_host::startup::mark("app: window set up");
    // 无边框 + 透明之后，窗口本身还是矩形，四角透出来的是桌面。把它裁成圆角矩形（半径和
    // UI 的 --screen-radius 对齐，见 crates/host/src/window_corner.rs）。
    if let Some(window) = app.get_webview_window("main") {
        // 圆角按窗口当前的缩放比例算（见 crates/host/src/window_corner.rs）：Windows 的缩放
        // 设置改了、或者窗口换了显示器，圆角跟着一起变，不会留下旧尺寸的裁剪区域。
        if let Ok(hwnd) = window.hwnd() {
            let scale = window.scale_factor().unwrap_or(1.0);
            westplay_host::window_corner::apply(hwnd.0 as isize, scale);
        }
    }
    // The companion's own window, watched as the cab's page is: a page whose process dies is loaded
    // again rather than left as a white window.
    watch_companion(app.handle().clone());
            // The QQ middle layer: a Python process of WestPlay's own, for QQ Music's page (qq_layer.rs).
            // Its folder is shipped beside the app; a build without it simply has no QQ Music data.
            let qq_layer = resources_folder(app.handle()).and_then(|resources| {
                westplay_host::qq_layer::Layer::start(&westplay_host::qq_layer::folder(&resources))
            });
            if let Some(layer) = &qq_layer {
                westplay_host::startup::mark(&format!(
                    "app: QQ layer at {}",
                    layer.folder().display()
                ));
            }
            // Kept by the app for as long as it runs: a local would be dropped at the end of this
            // setup closure, and its `Drop` ends the layer with it (which it did: the layer served for
            // a moment and then went).
            app.manage(QqLayer(std::sync::Mutex::new(qq_layer)));
            // NetEase Cloud Music's layer: the same shape, its own folder and port (`netease_layer.rs`).
            let netease_layer = resources_folder(app.handle()).and_then(|resources| {
                westplay_host::netease_layer::Layer::start(&westplay_host::netease_layer::folder(&resources))
            });
            if let Some(layer) = &netease_layer {
                westplay_host::startup::mark(&format!("app: NetEase layer on {}", layer.port()));
            }
            app.manage(NeteaseLayer(std::sync::Mutex::new(netease_layer)));
            // Both layers watched: one that stops answering is started again (`watch_layers`).
            watch_layers(app.handle().clone());
            // The tray: the closed window lives here. Its menu shows WestPlay again or quits it for good.
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{TrayIconBuilder, TrayIconEvent};
                // 初始文字是中文；页面一连上就用自己的语言改（`set_tray_words`）。
                let show = MenuItem::with_id(app, "show", "显示 WestPlay", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出 WestPlay", true, None::<&str>)?;
                if let Some(words) = app.try_state::<TrayWords>() {
                    if let Ok(mut kept) = words.0.lock() {
                        *kept = Some(TrayWordsInner { open: show.clone(), quit: quit.clone() });
                    }
                }
                let menu = Menu::with_items(app, &[&show, &quit])?;
                let icon = app
                    .default_window_icon()
                    .cloned()
                    .ok_or_else(|| "no window icon to show in the tray".to_string())?;
                TrayIconBuilder::with_id("westplay")
                    .icon(icon)
                    .tooltip("WestPlay")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                HIDDEN_BY_USER.store(false, std::sync::atomic::Ordering::Release);
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            westplay_host::log!("[quit] the tray says WestPlay quits");
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button, .. } = event
                            && button == tauri::tray::MouseButton::Left
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                HIDDEN_BY_USER.store(false, std::sync::atomic::Ordering::Release);
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
                westplay_host::log!("[tray] shown");
            }
            // A page that never says it's ready (it failed to load) still shows, rather than nothing at all.
            if let Some(window) = app.get_webview_window("main") {
                std::thread::spawn(move || {
                    std::thread::sleep(SHOW_ANYWAY);
                    if !HIDDEN_BY_USER.load(std::sync::atomic::Ordering::Acquire)
                        && !window.is_visible().unwrap_or(true)
                    {
                        westplay_host::startup::mark("app: window shown without its page ready");
                        let _ = window.show();
                    }
                });
            }
            let config_dir = app.path().app_config_dir()?;
            let file = SettingsFile::in_dir(&config_dir);
            let shared = Arc::new(SharedSettings::new(file.load()));
            let setup = SetupStore::load(&config_dir, app.handle());
            setup::watch_installed_mods(&setup);
            let customize = customize::Customize {
                dir: app.path().app_local_data_dir()?.join(westplay_host::customize::FOLDER),
                changed: Arc::default(),
            };
            let host = start_host(
                app.handle(),
                open_request.clone(),
                accounts.clone(),
                shared.clone(),
                setup.game(),
                Looks { config_dir: config_dir.clone(), customize_changed: customize.changed.clone() },
            )?;
            app.manage(customize);
            app.manage(SettingsStore { file, shared });
            app.manage(setup);
            app.manage(std::sync::Mutex::new(host));
            update::watch(app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the WestPlay app");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            stop_host(app);
        }
    });
}

/// Stops the host, so its WebView2 shuts down cleanly with the app (it quitting, or an update's installer
/// about to replace it). Once: a stopped host has nothing left to stop.
fn stop_host(app: &AppHandle) {
    if let Some(host) = app.try_state::<std::sync::Mutex<Host>>() {
        let mut host = host.lock().unwrap_or_else(|e| e.into_inner());
        host.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = host.thread.take() {
            let _ = thread.join();
        }
    }
}

/// What the host needs for the screen's look: where the settings are kept (the cab changes its wallpaper
/// itself), and the count of changes to the player's wallpapers and icon sets.
struct Looks {
    config_dir: PathBuf,
    customize_changed: Arc<std::sync::atomic::AtomicU64>,
}

/// How a run of the host ended.
enum Ended {
    Stopped,
    BrowserGone,
    /// To draw the cab on another GPU (westplay_host::graphics).
    GpuChanged,
    Failed(String),
}

/// The host is started again after its browser fails at most this often within the window, after a pause.
const MAX_RESTARTS: usize = 5;
const RESTART_WINDOW: Duration = Duration::from_secs(60);
const RESTART_DELAY: Duration = Duration::from_secs(1);
/// How long WebView2's browsers are waited for to go before the host starts on another GPU.
const BROWSERS_GOING: Duration = Duration::from_secs(10);

fn start_host(
    app: &AppHandle,
    open_request: Arc<OpenRequest>,
    accounts: Arc<AccountRequests>,
    settings: Arc<SharedSettings>,
    game: Arc<SetUpGame>,
    looks: Looks,
) -> Result<Host, Box<dyn std::error::Error>> {
    let content = cabin_content(app);
    let data_dir = app.path().app_local_data_dir()?;
    let map_builder = setup::map_builder_file(app);
    westplay_host::log!(
        "maps: {}",
        map_builder
            .as_ref()
            .map_or("no map builder: built in WestPlay itself".into(), |file| format!("built by {}", file.display()))
    );
    let stop = Arc::new(AtomicBool::new(false));

    let events = app.clone();
    let thread_stop = stop.clone();
    // The host runs on a thread of its own, started again (on a new thread) if WebView2's browser process
    // exits under it (westplay_host::BrowserGone): the cabin page comes back rather than staying blank. Not
    // endlessly: a browser failing again and again is reported instead.
    let thread = std::thread::Builder::new().name("westplay-host-supervisor".into()).spawn(move || {
        let mut restarts: Vec<Instant> = Vec::new();
        loop {
            let (content, data_dir) = (content.clone(), data_dir.clone());
            let (open_request, settings, stop) = (open_request.clone(), settings.clone(), thread_stop.clone());
            let (accounts, game, map_builder) = (accounts.clone(), game.clone(), map_builder.clone());
            let status_events = events.clone();
            let (settings_events, settings_dir) = (events.clone(), looks.config_dir.clone());
            let customize_changed = looks.customize_changed.clone();
            let run = std::thread::Builder::new().name("westplay-host".into()).spawn(move || {
                let config = Config {
                    width: SCREEN_WIDTH,
                    height: SCREEN_HEIGHT,
                    content,
                    data_dir,
                    open_request,
                    accounts,
                    settings,
                    game,
                    map_builder,
                    on_status: Box::new(move |status| {
                        let _ = status_events.emit("host-status", status);
                    }),
                    // The cab's own Settings changed one (its wallpaper): kept, and the companion told.
                    on_settings: Box::new(move |settings| {
                        if let Err(error) = SettingsFile::in_dir(&settings_dir).save(settings) {
                            westplay_host::log!("[settings] couldn't save the cab's change: {error}");
                        }
                        let _ = settings_events.emit("settings-changed", settings);
                    }),
                    customize_changed,
                };
                match westplay_host::run(config, stop) {
                    Ok(()) => Ended::Stopped,
                    Err(e) if e.is::<BrowserGone>() => Ended::BrowserGone,
                    Err(e) if e.is::<westplay_host::graphics::GpuChanged>() => Ended::GpuChanged,
                    Err(e) => Ended::Failed(e.to_string()),
                }
            });
            let ended = match run.map(|thread| thread.join()) {
                Ok(Ok(ended)) => ended,
                Ok(Err(_)) => Ended::Failed("it crashed".into()),
                Err(e) => Ended::Failed(e.to_string()),
            };
            let error = match ended {
                Ended::Stopped => return,
                Ended::BrowserGone | Ended::GpuChanged if thread_stop.load(Ordering::Relaxed) => return,
                // Started again on the other GPU once the browsers have gone (one still running would refuse
                // the new arguments). Not counted as a failure.
                Ended::GpuChanged => {
                    westplay_host::wait_for_browsers(BROWSERS_GOING);
                    continue;
                }
                Ended::BrowserGone => {
                    let now = Instant::now();
                    restarts.retain(|&at| now.duration_since(at) < RESTART_WINDOW);
                    restarts.push(now);
                    if restarts.len() <= MAX_RESTARTS {
                        std::thread::sleep(RESTART_DELAY);
                        continue;
                    }
                    BrowserGone.to_string()
                }
                Ended::Failed(error) => error,
            };
            // In the log too: shown only, a player's said nothing of it (2026-11-11).
            westplay_host::log!("WestPlay engine stopped: {error}");
            let status = Status { error: Some(format!("WestPlay engine stopped: {error}")), ..Default::default() };
            let _ = events.emit("host-status", &status);
            return;
        }
    })?;
    Ok(Host { stop, thread: Some(thread) })
}

/// The cabin page: from the Vite dev server while developing, from the bundled UI otherwise.
fn cabin_content(app: &AppHandle) -> Content {
    if cfg!(debug_assertions)
        && let Some(dev_url) = &app.config().build.dev_url
    {
        return Content::Url(format!("{}cabin.html", dev_url.as_str()));
    }
    let bundled =
        app.path().resource_dir().map(|dir| dir.join("ui")).ok().filter(|dir| dir.join("cabin.html").exists());
    // Unbundled release builds (`cargo build --release`) use the UI build next to the sources.
    let root = bundled.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/dist"));
    Content::Folder { root, page: "cabin.html".into() }
}
