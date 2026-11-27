//! WestPlay's game mod (`westplay.scs`), built on the player's PC from their own game: for each screen layout
//! (`westplay_shared::screen_layouts`), the game's GPS screen scripts with one image added as the top layer of
//! their "screen on" group, the screen texture WestPlay draws into. Nothing of the game's is shipped: the
//! scripts are the player's own copy's, patched, and the rest is WestPlay's.
//!
//! And the cab's mark in the dashboard scripts of the trucks the tablet goes in (`cab_mark`).
//!
//! Each layout's texture is SCREEN_WIDTH x (SCREEN_HEIGHT + the layout's index), so the plugin knows from its
//! size which kind of screen the game draws. It starts fully transparent: until the plugin fills it, or with
//! WestPlay off, the game's own screen shows through. A layout whose script the game doesn't have (a DLC the
//! player doesn't own) is left out.
//!
//! It isn't put in the game's `mod` folder or shown in its Mod Manager: it's written into WestPlay's own folder
//! (`places::westplay_documents`), and WestPlay's plugin mounts it into the game itself, first of all mods, in
//! every mode (single player, Convoy, TruckersMP). A copy in the game's `mod` folder is taken out
//! (`uninstall::remove_mod`): with both, the game has the screens' scripts twice and stops when a screen starts.
//!
//! The mod is a zip, as `.scs` mods may be. (Ported from the Python script that built it before, and checked
//! against it: the same patch byte for byte, and the same mod from the same game.)

use std::io::Write;
use std::path::{Path, PathBuf};

use westplay_mapdata::archive::Archive;
use westplay_mapdata::files::{GameFiles, Mount};
use westplay_mapdata::save;
use westplay_shared::{
    CAB_MARK_HEIGHT, CAB_MARK_WIDTH, PARKING_MARK_HEIGHT, PARKING_MARK_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH,
    ScreenLayout, TABLET, screen_layouts, tablet_layout,
};

use crate::cab_mark;
use crate::game::Kind;

/// The mod's file for the game `kind`, in WestPlay's own folder (`places::westplay_documents`): each game's own
/// (`westplay_shared::mod_file`).
pub fn file_name(kind: Kind) -> &'static str {
    westplay_shared::mod_file(kind.sdk_id())
}
/// Where the mod's screen materials and textures live in the game's file system.
const SCREEN_DIR: &str = "material/ui/westplay";
/// The screen's draw order: the game draws a script's elements by their layers, over all of it whatever group they're
/// in (the stock elements use layers 0 to 6), and WestPlay's is over every one of the game's but the parking camera's
/// ([`lift_parking_camera`]).
const SCREEN_LAYER: u32 = 100;
/// The "screen on" group's id in the GPS scripts.
const SCREEN_ON_GROUP: &str = "100";
/// The parking camera's group's id in the GPS scripts (the Coaches DLC's coaches: the camera behind the bus, shown
/// as it reverses or as the player turns it on).
const PARKING_CAMERA_GROUP: &str = "910";
/// The parking camera's mark's texture (`westplay_shared::PARKING_MARK_WIDTH`), in the camera's group: drawn only as
/// the game shows the camera, so the plugin can tell WestPlay's screen is under it.
const PARKING_MARK: &str = "parking_mark";
/// The draw order of WestPlay's tablet picture in YTHQ Tablet's scripts ([`TABLET_OVERLAY`]): under the truck screen's
/// ([`SCREEN_LAYER`]), both over YTHQ's own elements (theirs use layers 0 to 16). The plugin draws frames into only
/// one of the two, the app's "WestPlay on" setting deciding, and keeps the other's texture transparent: the one shines
/// through the other, whichever it is.
const TABLET_LAYER: u32 = SCREEN_LAYER - 1;
/// WestPlay's tablet picture in YTHQ Tablet's scripts: the `tablet` layout's texture, as the truck screen's picture is
/// the layout's of the screen it takes over. Its unit name is as long as the screen's (`_nameless.westplay.screen`,
/// which the game takes; the bridge's first, 27 characters long, was refused as malformed, 2026-11-16).
const TABLET_OVERLAY: &str = "_nameless.westplay.tablet";

/// The part of a YTHQ Tablet screen script's window its tablets' models show: the middle of it. Each of its models'
/// screen face (its third piece) covers that band of whatever texture its material samples (`u` 0.0001..0.9999, `v`
/// 0.1922..0.8078 of it, its archive's models 2026-11-22, its displays cropping the rest), and WestPlay's picture goes
/// on that part alone there, so its whole page shows on a tablet rather than the middle of it. Symmetrical about the
/// middle, so the same whichever way the game's rows run.
const YTHQ_TABLET_SHOWN: ShownPart = ShownPart { left: 0.0, right: 1.0, top: 0.1922, bottom: 0.8078 };

/// A screen of WestPlay's own making, for interiors the game gives the same screen script as others: the Smart
/// Dash Advanced (12.9") includes the same screens file as the 10.1" (`computers_2025.sui`, naming one GPS
/// script), so the mod couldn't tell them apart, and the larger screen got the smaller one's sizing. The
/// layout's script (its one, in `screen_layouts.txt`) is a copy of `source`, patched as the others are; its
/// interiors' definitions (the files in `interiors` whose name has `marker`), copies of the player's own with
/// their one `@include` of `screens` changed, include `own_screens` beside them: a copy of that screens file
/// whose `gps_path` names the copy. Built whole or not at all: if anything isn't as read (a game update), the
/// layout is left out, and those interiors keep the screen they share.
struct OwnScreen {
    layout: &'static str,
    source: &'static str,
    screens: &'static str,
    own_screens: &'static str,
    interiors: &'static [&'static str],
    marker: &'static str,
}

const OWN_SCREENS: &[OwnScreen] = &[OwnScreen {
    layout: "scania-2025-advanced",
    source: "ui/dashboard/scania_2025_gps.sii",
    screens: "computers_2025.sui",
    own_screens: "westplay_computers_2025_advanced.sui",
    interiors: &["def/vehicle/truck/scania.s_2016/interior", "def/vehicle/truck/scania.r_2016/interior"],
    marker: "smartdash_advanced",
}];

/// The part of a GPS script's screen its display shows, where the display shows only part of it: fractions of
/// the "screen on" group's width from the left and of its height from the top. WestPlay's image goes on that
/// part alone, so none of it is cut off and it isn't drawn out of shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShownPart {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

/// The parts shown, by layout. The stock GPS screen (`ui/gps.sii`, a 256 x 256 square) is WestPlay's only in
/// the Scania S and R's 7" display, which shows the square's full width (0.5%-99.5%) and rows 12.2%-73.5%
/// from its top: 254 x 157 of it, 1.615:1, as the interior's model maps its screen piece
/// (`scania_2016/interior/model.pmg`, piece 80, `gps.tobj`: PLAN.md section C). Stretched over the whole
/// square, the frame lost its bottom and top and was drawn 1.3 times too tall (owner 2026-10-21). The trucks' own
/// screens' displays each leave a thin edge of theirs out (their models' `gps.tobj` pieces, 2026-11-03; each
/// script's "screen on" group its whole window): the DAF 2021's and the Iveco S-Way's 0.25% of its width each
/// side, the Renault T 2024's 0.5% of every side, the Volvo FH's (2021 and 2024) 0.54% and 0.55%; the frame's
/// margin and round corners were cut by up to 7 pixels there. (The Smart Dash Advanced's 0.1% is under a pixel; the
/// Smart Dash's display shows all of its.) Each page is laid out in its part's shape (westplay_shared `page_height`).
const SHOWN_PARTS: &[(&str, ShownPart)] = &[
    ("generic", ShownPart { left: 0.005, right: 0.995, top: 0.122, bottom: 0.735 }),
    ("daf-2021", ShownPart { left: 0.0025, right: 0.9975, top: 0.0, bottom: 1.0 }),
    ("iveco-sway", ShownPart { left: 0.0025, right: 0.9975, top: 0.0, bottom: 1.0 }),
    ("renault-t-2024", ShownPart { left: 0.005, right: 0.995, top: 0.005, bottom: 0.995 }),
    ("volvo-fh", ShownPart { left: 0.0054, right: 0.9946, top: 0.0055, bottom: 0.9945 }),
    // The Volvo 9700 DD's centre display leaves 0.32% of the surface out on each edge (interior `gps.tobj`).
    ("volvo-9700-dd", ShownPart { left: 0.0032, right: 0.9968, top: 0.0032, bottom: 0.9968 }),
];

/// A screen of the game's own whose script is a bare window (no "screen on" group): WestPlay's image goes on the part
/// of the window the screen's glass shows (`shown`). The script is the game's, patched as the GPS screens' are, and
/// WestPlay shows on it only in its trucks (`westplay_shared::screen_supported`); a player without it (the DLC it's
/// from) gets a window of WestPlay's at its path. Nothing of the mod whose part shows it is changed.
struct WindowScreen {
    layout: &'static str,
    shown: ShownPart,
}

/// The game's toy GPS (Cabin Accessories: `gps.dlc_toys.sii`, a 300 x 168 window), the screen of Oyuncuyusbis' coaches'
/// Gps parts, drawn at the parts' own 512 x 256: WestPlay on them rather than on the buses' own screens (owner
/// 2026-10-26, the Setra S 516 HD/2's tablet: "way better"; "we should not change anything of the bus mod, just detect
/// that screen"; 2026-11-16: "all four busses"). Each part's glass shows its own part of the window
/// (`westplay_shared::glass`): the image covers the window whole, and the page goes on each bus's glass
/// (`westplay_shared::page_place`).
const WINDOW_SCREENS: &[WindowScreen] =
    &[WindowScreen { layout: "ob-gps", shown: ShownPart { left: 0.0, right: 1.0, top: 0.0, bottom: 1.0 } }];

/// The 7" Scania's screen drawn sharp. The interiors naming no GPS screen of their own (all but the Smart
/// Dash's) get the stock one at the game's default size: a 256 x 256 target (the 7" model's `gps.tobj` piece
/// was drawn with one: PLAN.md section C), into which WestPlay's 1280 x 720 frame was shrunk five times over,
/// and of which only 254 x 157 pixels show (owner 2026-10-21: "wtf is this resolution"). At 1024 x 1024
/// (square still, so the display shows the same part of it), about 1016 x 628 show, near the frame's own size.
/// Set as the Smart Dash's screens file sets its own (`gps_drawable_size`).
const SHARP_INTERIORS: &[&str] =
    &["def/vehicle/truck/scania.s_2016/interior", "def/vehicle/truck/scania.r_2016/interior"];
/// The interiors with a screen of their own, left as they are.
const SHARP_SKIP: &str = "smartdash";
/// Screens files that set the 7" screen's size themselves (`gps_drawable_size: (256, 256)` beside the stock
/// `gps_path`), included by every interior of their truck: the Scania S BEV's (the S's 7" display, its own
/// interiors: owner 2026-10-26). Each made [`SHARP_SIZE`] on its own (`sharp_screens_file`), apart from the S
/// and R's, so neither stops the other.
const SHARP_SCREENS: &[&str] = &["def/vehicle/truck/scania.s_2024e/interior/computers.sui"];
const SHARP_SIZE: &str = "gps_drawable_size: (1024, 1024)";
/// The Coaches DLC's screens files (every interior of their bus includes its own), each naming its bus's script
/// (the layouts `scania-touring`, `man-lions-24`) at 512 x 256: a 1280-wide frame halved and more, on displays the
/// size of the Volvo FH's. Made [`BUS_SIZE`], the Volvo FH's own (the same 2:1, so the displays show the same of
/// it), on their own, apart from each other (`sized_screens_file`).
const BUS_SCREENS: &[(&str, &str)] = &[
    ("def/vehicle/bus/scania.touring_23/interior/computers.sui", "ui/dashboard/scania_touring_23_gps.sii"),
    ("def/vehicle/bus/man.lions_24/interior/computers.sui", "ui/dashboard/man_lions_24_gps.sii"),
];
const BUS_SIZE: &str = "gps_drawable_size: (1024, 512)";
/// The stock GPS screen's script.
const STOCK_GPS: &str = "ui/gps.sii";

/// The 7" interiors' definitions, each the player's own with [`SHARP_SIZE`] added after its `dashboard_path`.
/// Whole or not at all: an interior not as read (naming a GPS screen or its size already: a game update)
/// leaves them all at the game's size.
fn sharp_screen_files(game: &GameFiles) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files = Vec::new();
    for dir in SHARP_INTERIORS {
        let names = game.list(dir).map_err(|error| error.to_string())?;
        for name in names.iter().filter(|name| name.ends_with(".sii") && !name.contains(SHARP_SKIP)) {
            let path = format!("{dir}/{name}");
            let bytes = game.read(&path).map_err(|error| error.to_string())?.ok_or(format!("no {path}"))?;
            let text = String::from_utf8(bytes).map_err(|_| format!("{path}: not UTF-8 text"))?;
            // The stock screen, named or not (the exclusive interiors, brown, wood and V8, name it).
            let other_screen = text.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("gps_path") && !line.contains(&format!("\"/{STOCK_GPS}\""))
            });
            if other_screen || text.contains("gps_drawable_size") {
                return Err(format!("{path}: names a GPS screen of its own, or its size"));
            }
            let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
            let mut lines: Vec<String> = text.split(newline).map(str::to_owned).collect();
            let found: Vec<usize> =
                (0..lines.len()).filter(|&at| lines[at].trim_start().starts_with("dashboard_path")).collect();
            let [at] = found[..] else {
                return Err(format!("{path}: {} dashboard_path lines, not one", found.len()));
            };
            let indent = lines[at][..lines[at].len() - lines[at].trim_start().len()].to_owned();
            lines.insert(at + 1, format!("{indent}{SHARP_SIZE}"));
            files.push((path, lines.join(newline).into_bytes()));
        }
    }
    if files.is_empty() {
        return Err("no 7\" interiors".into());
    }
    Ok(files)
}

/// A screens file of [`SHARP_SCREENS`] with its one stock-screen size made [`SHARP_SIZE`] ([`sized_screens_file`]).
fn sharp_screens_file(game: &GameFiles, path: &str) -> Result<Option<Vec<u8>>, String> {
    sized_screens_file(game, path, STOCK_GPS, SHARP_SIZE)
}

/// The screens file at `path` with its one GPS screen's size made `size` (a `gps_drawable_size` line); `None` if
/// the game hasn't it (its DLC not owned). Not as read (a screen named other than `script`, or no size or two): an
/// error, the file left as the game has it.
fn sized_screens_file(game: &GameFiles, path: &str, script: &str, size: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(bytes) = game.read(path).map_err(|error| error.to_string())? else { return Ok(None) };
    let text = String::from_utf8(bytes).map_err(|_| format!("{path}: not UTF-8 text"))?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split(newline).map(str::to_owned).collect();
    let starting = |name: &str| -> Vec<usize> {
        (0..lines.len()).filter(|&at| lines[at].trim_start().starts_with(name)).collect()
    };
    let (screens, sizes) = (starting("gps_path"), starting("gps_drawable_size"));
    let ([screen], [size_at]) = (&screens[..], &sizes[..]) else {
        return Err(format!(
            "{path}: {} gps_path and {} gps_drawable_size lines, not one each",
            screens.len(),
            sizes.len()
        ));
    };
    if !lines[*screen].contains(&format!("\"/{script}\"")) {
        return Err(format!("{path}: names another GPS screen than {script}"));
    }
    let indent = lines[*size_at][..lines[*size_at].len() - lines[*size_at].trim_start().len()].to_owned();
    lines[*size_at] = format!("{indent}{size}");
    Ok(Some(lines.join(newline).into_bytes()))
}

/// An own screen's files (`own`, its layout's `script`, its texture `texture`, `rows` tall), from `game`: the
/// script's copy, and per interiors' folder the screens file's copy and the interiors' definitions.
fn own_screen_files(
    game: &GameFiles,
    own: &OwnScreen,
    script: &str,
    texture: &str,
    rows: u32,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let read = |path: &str| -> Result<String, String> {
        let bytes = game.read(path).map_err(|error| error.to_string())?.ok_or(format!("no {path}"))?;
        String::from_utf8(bytes).map_err(|_| format!("{path}: not UTF-8 text"))
    };
    let mut files = vec![(script.to_owned(), patch_gps_script(&read(own.source)?, texture, None, rows)?.into_bytes())];
    let replace_once = |text: &str, from: &str, to: &str, path: &str| -> Result<String, String> {
        match text.matches(from).count() {
            1 => Ok(text.replacen(from, to, 1)),
            count => Err(format!("{path}: {count} times {from}, not once")),
        }
    };
    let mut interiors = 0;
    for dir in own.interiors {
        let screens_path = format!("{dir}/{}", own.screens);
        let screens = replace_once(
            &read(&screens_path)?,
            &format!("\"/{}\"", own.source),
            &format!("\"/{script}\""),
            &screens_path,
        )?;
        files.push((format!("{dir}/{}", own.own_screens), screens.into_bytes()));
        let names = game.list(dir).map_err(|error| error.to_string())?;
        for name in names.iter().filter(|name| name.contains(own.marker) && name.ends_with(".sii")) {
            let path = format!("{dir}/{name}");
            let include = |file: &str| format!("@include \"{file}\"");
            let text = replace_once(&read(&path)?, &include(own.screens), &include(own.own_screens), &path)?;
            files.push((path, text.into_bytes()));
            interiors += 1;
        }
    }
    if interiors == 0 {
        return Err(format!("no interiors named {}", own.marker));
    }
    Ok(files)
}

/// What a build made: the layouts patched, and those whose scripts the game doesn't have; why the 7" Scania
/// screen stays at the game's size, if it does (its interiors not as read: `sharp_screen_files`); how many
/// dashboard scripts have the cab's mark; and which of YTHQ Tablet's own screen scripts are in it, patched for the
/// factory screens they take the game's place on ([`ythq_screens`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Built {
    pub patched: Vec<&'static str>,
    pub missing: Vec<&'static str>,
    pub not_sharp: Option<String>,
    pub cab_marks: usize,
    /// YTHQ Tablet's screen scripts the mod carries, patched: its `gps_path` the trucks' factory screens would load
    /// with YTHQ Tablet installed (none if it isn't).
    pub ythq: Vec<&'static str>,
    /// The tablet-only copies of YTHQ Tablet's screen scripts the mod carries ([`YTHQ_TABLET_DIR`]): WestPlay's tablet
    /// picture on them, which its tablets draw ([`ythq_accessories`] have their `ui_path`s). European Truck Simulator
    /// 2's alone: American Truck Simulator's one screen script carries both pictures itself, which its tablets draw as
    /// its trucks' screens do (none).
    pub ythq_tablets: Vec<String>,
    /// YTHQ Tablet's tablets' definitions the mod carries, their `ui_path`s the copies'. European Truck Simulator 2's
    /// alone: American Truck Simulator's tablets keep its own (none), their `ui_path`s the script the mod patches at
    /// its own path.
    pub ythq_accessories: Vec<String>,
    /// YTHQ Tablet's tablets' models' screen materials the mod carries copies of: its automat materials their screens
    /// sample. European Truck Simulator 2's pointed at its tablets' own UI texture ([`YTHQ_TABLET_UI`], which the
    /// tablets' scripts' copies draw into); American Truck Simulator's kept as YTHQ has them (the trucks' own screens'
    /// texture, [`YTHQ_FACTORY_UI`]) but carried at paths of the mod's own: its YTHQ Tablet's screen materials are the
    /// Ford cars' too (`ats-ythq-ford`: their screens sample WestPlay's tablet texture there).
    pub ythq_tablet_materials: Vec<String>,
    /// Why some of YTHQ Tablet's screen scripts aren't in the mod, if any aren't: a version without one, or one laid
    /// out otherwise than [`patch_gps_script`] expects. None if YTHQ Tablet isn't installed, or all are in.
    pub ythq_problem: Option<String>,
}

/// Why the mod couldn't be built.
#[derive(Debug)]
pub enum BuildError {
    /// A script couldn't be read from the game.
    Read(String),
    /// A script isn't laid out as WestPlay expects (a new game version, or a mod's own).
    Script { script: &'static str, problem: String },
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(f, "{error}"),
            Self::Script { script, problem } => write!(f, "{script}: {problem}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// The add-ons WestPlay brings on the vehicles a patch of its own makes WestPlay's: YTHQ's tablet trucks in both
/// games, the Ford F-150 and Bronco's centre screens in American Truck Simulator, and the YTHQ tablets of the
/// workshop's Ford Mustang '67 and Ford Victoria '06 cars (whose models' screen faces showed WestPlay's frame
/// cut and upside down, their UVs sampling YTHQ's old dashboard texture's middle band: `ats-ythq-ford`), kept
/// byte for byte as their makers built them (owner 2026-11-16; the cars 2026-11-17), save American Truck
/// Simulator's YTHQ tablet models, left out (YTHQ's own show its tablets' UI: [`ythq_screen_files`]).
const ADDONS: &[u8] = include_bytes!("../assets/patches.zip");

/// `script` with the add-ons' opaque backdrop made clear: the Fords' centre screens carry a black fill at layer 100
/// under WestPlay's own picture, put there so none of SYNC 4 shows through. Left clear, their own screen shows
/// whenever WestPlay isn't drawing there (the app closed, or Show WestPlay off) and WestPlay's covers it while it
/// does (owner 2026-11-16: the Ford's own screen put back while WestPlay isn't drawing).
fn clear_backdrop(script: Vec<u8>) -> Vec<u8> {
    const DARK: &str = "<color value=FF000000><img src=/material/ui/white.mat";
    const CLEAR: &str = "<color value=00000000><img src=/material/ui/white.mat";
    let Ok(mut text) = String::from_utf8(script.clone()) else { return script };
    if text.contains(DARK) {
        text = text.replace(DARK, CLEAR);
    }
    let text = inset_overlay(&lift_parking(&text), YTHQ_INSET_X, YTHQ_INSET_Y);
    text.into_bytes()
}

/// How much smaller WestPlay's picture is drawn on a YTHQ tablet than the add-on's script draws it: their models show
/// it a size too big, on both the large and the small tablet (owner 2026-11-16, a Volvo FH 2024: "大了一圈").
const YTHQ_INSET_X: f64 = 0.95;
/// Vertically the models show less of the texture than they do across it: the picture overflows above and below
/// while its sides fit (owner 2026-11-16, a Volvo FH 2024's tablet: "左右两边还行 上下就不行了").
const YTHQ_INSET_Y: f64 = 1.0;

/// `script` with WestPlay's picture on a YTHQ tablet (`_nameless._.wpb`) scaled about its own middle by `factor`, so the
/// picture fits inside the tablet. A script without one is left as it is.
fn inset_overlay(script: &str, across: f64, down: f64) -> String {
    let lines: Vec<&str> = script.split_inclusive('\n').collect();
    let mut out = String::with_capacity(script.len());
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if !(line.trim_start().starts_with("ui::") && line.contains("wpb")) {
            out.push_str(line);
            index += 1;
            continue;
        }
        // The block runs to the line that is the closing brace alone.
        let start = index;
        let mut end = index + 1;
        while end < lines.len() && lines[end].trim() != "}" {
            end += 1;
        }
        let end = end.min(lines.len().saturating_sub(1));
        let block = &lines[start..=end];
        let value = |key: &str| -> Option<f64> {
            block.iter().find_map(|line| {
                let (name, rest) = line.trim().split_once(':')?;
                if name != key {
                    return None;
                }
                rest.trim().parse::<f64>().ok()
            })
        };
        let middle = match (value("coords_l"), value("coords_r"), value("coords_t"), value("coords_b")) {
            (Some(left), Some(right), Some(top), Some(bottom)) => Some(((left + right) / 2.0, (top + bottom) / 2.0)),
            _ => None,
        };
        for line in block {
            let Some((middle_x, middle_y)) = middle else {
                out.push_str(line);
                continue;
            };
            let trimmed = line.trim_start();
            let Some((key, rest)) = trimmed.split_once(':') else {
                out.push_str(line);
                continue;
            };
            let Ok(value) = rest.trim().parse::<f64>() else {
                out.push_str(line);
                continue;
            };
            let (centre, factor) = match key {
                "coords_l" | "coords_r" => (middle_x, across),
                "coords_t" | "coords_b" => (middle_y, down),
                _ => {
                    out.push_str(line);
                    continue;
                }
            };
            let scaled = centre + (value - centre) * factor;
            let indent = &line[..line.len() - trimmed.len()];
            let newline = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!("{indent}{key}: {scaled:.0}{newline}"));
        }
        index = end + 1;
    }
    out
}

/// `script` with its parking camera's elements lifted over WestPlay's picture: the reversing camera shows over it,
/// as the game draws it over its own screen (owner 2026-11-16). A script with no camera, or whose layers aren't
/// numbers, is left as it is.
fn lift_parking(script: &str) -> String {
    const ABOVE: u32 = 100;
    let mut out = String::with_capacity(script.len());
    let mut camera = false;
    for line in script.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("ui::") {
            camera = trimmed.contains("park");
            out.push_str(line);
            continue;
        }
        if camera
            && let Some(rest) = trimmed.strip_prefix("layer: ")
            && let Ok(layer) = rest.trim().parse::<u32>()
        {
            let indent = &line[..line.len() - trimmed.len()];
            let newline = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!("{indent}layer: {}{newline}", layer + ABOVE));
            continue;
        }
        out.push_str(line);
    }
    out
}

/// A number naming what [ADDONS] holds, for the app to know a mod built before it as out of date: the archive's
/// bytes hashed (owner 2026-11-16: the plate's patches of YTHQ's and the Fords' screens, added to the mod as they
/// are).
pub fn addons_digest() -> u64 {
    use std::hash::{Hash, Hasher};
    // Bumped whenever the code turns the add-ons into files (a script made clear, a path changed): a mod built with
    // the older turn of them is made again. 14: American Truck Simulator's YTHQ Tablet keeps its own tablets'
    // definitions (`ui_path`, `ui_drawable_texture_path`: the script the mod patches in place, the trucks' own screens'
    // texture) and its own tablet models (the add-ons' left out), and its tablets' screen materials sample the trucks'
    // own screens' texture again, at paths of the mod's own: built before them, its tablets' screens showed WestPlay's
    // tablet picture directly, YTHQ's UI never drawn there (the tablets black while the app's "truck screen" is
    // chosen). 13: American Truck Simulator's YTHQ Tablet's own one screen script is read and patched
    // (`YTHQ_SCREENS_ATS`), its tablets' definitions pointed at copies of it, their screen materials at its
    // tablets' own UI texture (`ythq_tablet_materials`), at paths of the mod's own where YTHQ's are other vehicles'
    // too: a mod built before them has no UI addon of its own (the game failed to load it, the tablets' screens then
    // black), or leaves YTHQ's own UI when the app's "truck screen" is chosen. 12: YTHQ Tablet's tablets' definitions
    // and their models' screen materials are pointed at its tablets' own UI texture
    // (`YTHQ_TABLET_UI`, `ythq_tablet_materials`), not the trucks' own screens' (`/vehicle/truck/share/gps.tobj`, which
    // both screens' models sample: built before them, "truck screen" chosen showed WestPlay's picture on the tablets
    // and "WestPlay tablet" left them YTHQ's UI with none of WestPlay's). 11: YTHQ Tablet's tablets have screen scripts
    // of their own (`YTHQ_TABLET_DIR`, `ythq_accessories`): with the app's "truck screen" chosen they show YTHQ's own
    // UI, and WestPlay's tablet picture in the app's "WestPlay tablet" one, where a mod built before them had the truck
    // screen's picture on the factory screens' scripts (and so on the tablets too, stretched over them). 10: YTHQ
    // Tablet's own screen scripts carry WestPlay's tablet picture as well as the truck screen's
    // (`patch_ythq_tablet_script`): the app's "WestPlay tablet" shows on YTHQ Tablet's screen, built before them as its
    // own UI. 9: YTHQ Tablet's own screen scripts, read from its archive and patched for the trucks' factory screens
    // (see `ythq_screens`): a mod built before them keeps YTHQ's screen instead. 8: the Ford Mustang '67 and Victoria
    // '06's YTHQ tablet models (`ats-ythq-ford`), whose screen faces were stretched over WestPlay's tablet texture.
    // 15: European Truck Simulator 2's YTHQ Tablet pictures go on the part of their windows YTHQ's tablet screens
    // show, as American Truck Simulator's do (owner 2026-11-22: the whole window was the wrong shape).
    const FORMAT: u64 = 15;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ADDONS.hash(&mut hasher);
    FORMAT.hash(&mut hasher);
    hasher.finish()
}

/// The add-ons of `kind`'s game ([ADDONS]), by their paths in it: their own `manifest.sii`, `description.txt` and
/// `readme.txt` left out (the mod's manifest is WestPlay's).
fn addons(kind: crate::game::Kind) -> Vec<(String, Vec<u8>)> {
    let groups: &[&str] = match kind {
        crate::game::Kind::Ets2 => &["ets2-ythq", "ets2-phones"],
        crate::game::Kind::Ats => &["ats-ythq", "ats-ythq-ford", "ats-ford"],
    };
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(ADDONS)) else { return Vec::new() };
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(index) else { continue };
        let name = entry.name().to_owned();
        let Some((group, path)) = name.split_once('/') else { continue };
        if !groups.contains(&group) || path.is_empty() || matches!(path, "manifest.sii" | "description.txt" | "readme.txt") {
            continue;
        }
        let mut bytes = Vec::new();
        if std::io::Read::read_to_end(&mut entry, &mut bytes).is_err() {
            continue;
        }
        // American Truck Simulator's YTHQ tablet models: not carried. Its YTHQ Tablet's own models show the band of the
        // trucks' own screens' texture their screens were made for (`YTHQ_TABLET_SHOWN`), which its tablets' screen
        // materials sample again (`ythq_screen_files`), where these were laid out for WestPlay's tablet texture itself
        // (the whole of it, its `v` the other way): a tablet showing them would show the texture's middle only, or the
        // trucks' UI upside down. The cars' YTHQ tablets (`ats-ythq-ford`) are its others: theirs stay, their own
        // screens' materials sampling WestPlay's tablet texture.
        if group == "ats-ythq" && path.ends_with(".pmg") {
            continue;
        }
        files.push((path.to_owned(), clear_backdrop(bytes)));
    }
    files
}

/// YTHQ Tablet's Workshop item id, per game: its folder in that game's Workshop content folder
/// (`Game::workshop`). The mod is published for each game under an id of its own: an ATS install's Workshop
/// folder has the ATS item (`3598886740`), never the ETS2 one (`3595920707`), so the id has to follow the game.
fn ythq_item(kind: crate::game::Kind) -> &'static str {
    match kind {
        crate::game::Kind::Ets2 => "3595920707",
        crate::game::Kind::Ats => "3598886740",
    }
}

/// YTHQ Tablet's own screen scripts, per game, and the layout of the truck screen each takes the game's place on: its
/// copies of the game's `def/vehicle/truck/*/interior/computers*.sui` name them as the factory screen's `gps_path`, so
/// with YTHQ Tablet installed the game never loads the game's own scripts the mod patches. Each is patched as the
/// game's script of that screen is ([`patch_gps_script`]: WestPlay's picture on the part the display shows, or on its
/// window whole: [`ythq_shown`]) and carried in this mod at the same path, which is mounted over YTHQ's. Its tablets
/// (UI accessories whose `ui_path` names these same scripts) are pointed at copies of their own ([`YTHQ_TABLET_DIR`]:
/// the app's "WestPlay tablet" picture there, YTHQ's own UI otherwise), on a screen texture of their own
/// ([`YTHQ_TABLET_UI`]): the factory screens keep the truck screens' pictures alone. American Truck Simulator's one
/// script is its tablets' too, as YTHQ has them (both its displays draw the trucks' own screens' texture): WestPlay's
/// tablet picture goes on it ([`YTHQ_TABLET_SHOWN`]), its definitions and models kept.
///
/// ETS2's (read from YTHQ Tablet, 2026-11-18): the Volvo FH 2021 and 2024's km/h and mph screens (one script each),
/// the Renault T 2024's (its `computers_2024*.sui`), the Iveco S-Way's, and the Scania S and R's Smart Dash 10.1"
/// (`scania-2025`, its `computers_2025*.sui`; the Scania S BEV's 7" screen names the same script but the mod makes
/// its screens file's size its own, `SHARP_SCREENS`, which is mounted over YTHQ's). Their `gps_drawable_size`s stay
/// YTHQ's (a render target's resolution: the texture's size and so the layout the plugin knows it by is ours). The MAN
/// TGX 2020 shares the Iveco S-Way's script (`_h_nfb`) and its own factory screen isn't a layout of WestPlay's, so
/// WestPlay stays off that one (`westplay_shared::SHARED_LAYOUT_TRUCKS`); its YTHQ tablet shows WestPlay's tablet
/// layout still, as does the tablet of every truck whose YTHQ tablet accessory names one of these scripts (the game's
/// own scripts keep their factory screens).
const YTHQ_SCREENS_ETS2: &[(&str, &str)] = &[
    ("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii", "volvo-fh"),
    ("ui/dashboard/uk_ythq_tablet_gps_mirror_nfb.sii", "volvo-fh"),
    ("ui/dashboard/ythq_tablet_gps_mirror_h_nfb.sii", "iveco-sway"),
    ("ui/dashboard/uk_ythq_tablet_gps_mirror_h_nfb.sii", "iveco-sway"),
    ("ui/dashboard/ythq_tablet_gps_mirror.sii", "scania-2025"),
    ("ui/dashboard/uk_ythq_tablet_gps_mirror.sii", "scania-2025"),
    ("ui/dashboard/ythq_renault_t_2024_gps.sii", "renault-t-2024"),
    ("ui/dashboard/uk_ythq_renault_t_2024_gps.sii", "renault-t-2024"),
];

/// American Truck Simulator's YTHQ Tablet (read from its own Workshop item, 2026-11-22, not ETS2's): a different mod,
/// with one screen script for every truck. Its `def/vehicle/truck/*/interior/computers*.sui` name
/// `ythq_tablet_gps_mirror_h.sii` as the factory screen's `gps_path`, and every one of its tablets (its definitions'
/// `ui_path`, `def/vehicle/truck/*/accessory/{toyac,set_glass,toystand,set_dashbrd}/*.sii`) names the same script and
/// draws it into the trucks' own screens' texture (`/vehicle/truck/share/gps.tobj`, which their models' screens
/// sample): its 1024 x 1024 window is the whole surface both its displays show, its tablets' screen faces covering the
/// middle band of it ([`YTHQ_TABLET_SHOWN`], its `ui_drawable_size`s (1024, 1024)). ETS2's names (`_nfb`, `_h_nfb`,
/// the `uk_` ones, the Renault T 2024's) aren't in its files at all. Its layout is the stock GPS screen's (`generic`,
/// `ui/gps.sii`), whose script YTHQ's takes the factory screens' place over; but American Truck Simulator's own screens
/// aren't layouts of WestPlay's (`westplay_shared::SHARED_LAYOUT_TRUCKS`), so its factory screens keep YTHQ's picture
/// and its tablets show WestPlay's only where the app's "WestPlay tablet" is chosen: WestPlay's tablet picture goes on
/// that one script, on the part its tablets' screens show.
const YTHQ_SCREENS_ATS: &[(&str, &str)] = &[("ui/dashboard/ythq_tablet_gps_mirror_h.sii", "generic")];

/// YTHQ Tablet's screen scripts of the game `kind` ([`YTHQ_SCREENS_ETS2`], [`YTHQ_SCREENS_ATS`]).
fn ythq_screens(kind: crate::game::Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        crate::game::Kind::Ets2 => YTHQ_SCREENS_ETS2,
        crate::game::Kind::Ats => YTHQ_SCREENS_ATS,
    }
}

/// The part of a YTHQ Tablet screen script's window its display shows: the part the game's own screen of `layout`
/// shows ([`SHOWN_PARTS`]), where YTHQ lays the script out as that screen (ETS2's, one script a screen), or the window
/// whole (American Truck Simulator's one script: its trucks' displays show all of its 1024 x 1024 surface).
fn ythq_shown(kind: crate::game::Kind, layout: &str) -> Option<&'static ShownPart> {
    match kind {
        crate::game::Kind::Ets2 => SHOWN_PARTS.iter().find(|(id, _)| *id == layout).map(|(_, part)| part),
        crate::game::Kind::Ats => None,
    }
}

/// Where the mod carries its own copies of YTHQ Tablet's screen scripts for its tablets alone: each script a tablet's
/// definition names ([`ythq_screens`], [`YTHQ_TABLETS`]) has one here, with WestPlay's tablet picture on it and nothing
/// of the truck screen's. Those definitions ([`ythq_accessories`]) are pointed at them, so a tablet shows the app's
/// "WestPlay tablet" picture where it's chosen and YTHQ's own UI otherwise: the plugin fills the tablet layout's
/// texture there alone, and clears it as the app's "truck screen" is chosen. YTHQ's factory screen scripts keep the
/// truck screens' picture alone ([`patch_gps_script`]), which its tablets never draw: its tablets cover them. European
/// Truck Simulator 2's alone: American Truck Simulator's tablets draw the factory screens' script itself
/// ([`ythq_screen_files`]).
const YTHQ_TABLET_DIR: &str = "ui/dashboard/westplay_ythq";

/// The texture object YTHQ Tablet's tablets draw the UI their scripts have into, and their models' screen materials
/// sample: its own, the one its tablets of the old trucks already have (owner 2026-11-18: with the trucks' own
/// screen's, `/vehicle/truck/share/gps.tobj`, both a truck screen's model and a YTHQ tablet's sample it, and the game
/// draws the factory screen's script into it: "truck screen" chosen showed WestPlay's own picture stretched over the
/// tablets, and "WestPlay tablet" left them YTHQ's UI with none of WestPlay's). Its tablets' definitions are pointed at
/// this one, and their models' screen materials too ([`ythq_tablet_materials`]), leaving the factory screens' own.
/// European Truck Simulator 2's alone: American Truck Simulator's tablets keep drawing the trucks' own screens' texture
/// ([`YTHQ_FACTORY_UI`]), its one script carrying WestPlay's tablet picture itself.
const YTHQ_TABLET_UI: &str = "vehicle/truck/upgrade/interior_decors/toyac/ythq_tablet_ui.tobj";
/// The trucks' own screens' texture object: what the game's interiors' screen materials have, and what YTHQ Tablet's
/// tablets' models' materials had before [`YTHQ_TABLET_UI`] (both screens' models sampling it is why a tablet showed
/// the factory screen's picture).
const YTHQ_FACTORY_UI: &str = "vehicle/truck/share/gps.tobj";

/// YTHQ Tablet's screen scripts its tablets' definitions name besides [`ythq_screens`]: its tablets of the trucks
/// whose factory screen isn't WestPlay's (the old Scanias', the Actros MP2's) show `ythq_tablet_gps.sii`, and the
/// coaches' (the MAN Lion's Coach's, the Scania Touring's, the Volvo 9700's) `ythq_tablet_gps_mirror_bus.sii`, which is
/// their factory screen too (the coaches' own screens being ones WestPlay patches, only the Volvo 9700's factory screen
/// can still load it: YTHQ's own UI then, the add-ons' copies of these two left out of the mod as their tablets draw
/// copies of their own, so no WestPlay picture of the tablet's goes on a factory screen). Tablet copies alone are made
/// of these ([`YTHQ_TABLET_DIR`]): nothing of them is patched for a factory screen. ETS2's tablets alone: American
/// Truck Simulator's name none of these.
const YTHQ_TABLETS: &[&str] = &["ui/dashboard/ythq_tablet_gps.sii", "ui/dashboard/ythq_tablet_gps_mirror_bus.sii"];

/// Where YTHQ Tablet's tablets' definitions are, in its archive: each vehicle's accessories, a folder a slot (`toyac`,
/// the tablet on the dash; `set_glass`, the one on the windscreen), a file a tablet (`ui_path`).
const YTHQ_VEHICLES: &[&str] = &["def/vehicle/truck/", "def/vehicle/bus/"];

/// YTHQ Tablet's screens as a build takes them: its own screens' scripts patched for the truck screens they take the
/// game's place on (the truck's picture on them). European Truck Simulator 2's also get copies of its screen scripts
/// for its tablets alone ([`YTHQ_TABLET_DIR`]), its tablets' definitions pointed at them and their models' screen
/// materials at its tablets' own UI texture ([`YTHQ_TABLET_UI`]); American Truck Simulator's one screen script carries
/// WestPlay's tablet picture itself, and its tablets keep its own definitions and models, their screens' materials as
/// YTHQ has them (the trucks' own screens' texture, [`YTHQ_FACTORY_UI`]). Nothing of its if it isn't installed.
#[derive(Default)]
struct YthqScreens {
    /// What the mod carries of its, by path in the game.
    files: Vec<(String, Vec<u8>)>,
    /// Its factory screens' scripts, patched for the layouts of the screens they take over.
    screens: Vec<&'static str>,
    /// The tablet-only copies of its screen scripts.
    tablets: Vec<String>,
    /// Its tablets' definitions, their `ui_path`s the copies'.
    accessories: Vec<String>,
    /// Its tablets' models' screen materials, carried at paths of the mod's own.
    materials: Vec<String>,
    /// Its tablets' models whose screens sample its screens' texture (YTHQ's own models: the add-ons' of them are left
    /// out of the mod, [`addons`]).
    models: Vec<String>,
    /// Why some of it isn't in the mod (a version laid out otherwise, its archive not read), if any.
    problems: Vec<String>,
}

/// The archives in `workshop` (the game's Workshop content folder) that YTHQ Tablet ships in: its item's `.scs`
/// (or `.zip`) files. Empty if it isn't installed.
fn ythq_archives(workshop: Option<&Path>, kind: crate::game::Kind) -> Vec<Archive> {
    let Some(item) = workshop.map(|workshop| workshop.join(ythq_item(kind))) else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(&item) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "scs" || extension == "zip"))
        .collect();
    paths.sort();
    paths.iter().filter_map(|path| Archive::open(path).ok()).collect()
}

/// YTHQ Tablet's screen scripts of the game `kind` ([`ythq_screens`]) each patched for the layout of the screen it
/// takes over (WestPlay's picture on the part of it the display shows, as the game's own script has). European Truck
/// Simulator 2's also get copies of their own for its tablets ([`YTHQ_TABLET_DIR`], WestPlay's tablet picture on them
/// whole, [`YTHQ_TABLET_UI`] the texture they draw into), its tablets' definitions pointed at them. American Truck
/// Simulator's one screen script carries WestPlay's tablet picture itself, on the part of its window its tablets'
/// screens show ([`YTHQ_TABLET_SHOWN`]), and its tablets keep its own definitions (their `ui_path` that script, their
/// `ui_drawable_texture_path` the trucks' own screens' texture). Quietly none if YTHQ Tablet isn't installed (nothing
/// of its to take over), and quietly the rest if any of its scripts aren't as expected; European Truck Simulator 2's
/// tablets' definitions not read as expected leave its scripts as the mod had them before the copies (WestPlay's
/// tablet picture on the factory screens' scripts too, which its tablets draw), so that no tablet goes blank.
///
/// Its tablets' models' screen materials are carried at paths of the mod's own ([`ythq_tablet_material_path`]: YTHQ's
/// own are other vehicles' too, the Ford cars' of `ats-ythq-ford`, whose screens sample WestPlay's tablet texture
/// there), pointed at its tablets' own UI texture in European Truck Simulator 2 ([`YTHQ_TABLET_UI`]: a tablet's own
/// screen then shows the UI its copy's script has, not the factory screen's picture, which both screens' models sample
/// as long as they all sample the trucks' own screen's texture), and kept as YTHQ has them in American Truck Simulator
/// ([`YTHQ_FACTORY_UI`]: its tablets draw what the game draws on the trucks' screens, YTHQ's own UI and WestPlay's
/// picture over it, its own models and definitions kept).
fn ythq_screen_files(workshop: Option<&Path>, kind: crate::game::Kind) -> YthqScreens {
    let archives = ythq_archives(workshop, kind);
    if archives.is_empty() {
        return YthqScreens::default();
    }
    let mut made = YthqScreens::default();
    // American Truck Simulator's YTHQ Tablet: its one screen script is the trucks' factory screens' and its tablets'
    // alike (its tablets' definitions draw it into the trucks' own screens' texture), so WestPlay's tablet picture goes
    // on that script, on the part its tablets' screens show, and its tablets keep its own definitions. Copies of the
    // script for the tablets alone, as European Truck Simulator 2's have, are what a player's truck showed black on
    // them (2026-11-22: the game draws the screens it has, and none of YTHQ's UI was drawn into a tablet's own copy,
    // WestPlay's picture over nothing while the app's "truck screen" left the copy clear).
    let together = matches!(kind, crate::game::Kind::Ats);
    // WestPlay's tablet picture's texture, which the plugin knows the tablet layout by (its size).
    let tablet = texture_name(&screen_layouts()[tablet_layout()]);
    // Its own screens' scripts as its archive has them: the script, the layout of the screen it takes over, the text.
    let mut screens: Vec<(&'static str, &'static str, String)> = Vec::new();
    for &(script, layout) in ythq_screens(kind) {
        if !screen_layouts().iter().any(|found| found.id == layout) {
            made.problems.push(format!("{script}: no layout {layout}"));
            continue;
        }
        match ythq_text(&archives, script) {
            Ok(Some(text)) => screens.push((script, layout, text)),
            Ok(None) => made.problems.push(format!("{script}: not in YTHQ Tablet's files")),
            Err(problem) => made.problems.push(format!("{script}: {problem}")),
        }
    }
    // Its tablets' definitions, the scripts and the models they name: each is pointed at that script's copy.
    let tablets = ythq_accessories(&archives, kind);
    let mut split = tablets.problems.is_empty();
    made.problems.extend(tablets.problems.iter().cloned());
    // Its tablets' own scripts (not its factory screens'): copied as well, if its tablets name them.
    let mut tablets_only: Vec<(String, String)> = Vec::new();
    for &script in YTHQ_TABLETS.iter().filter(|script| tablets.named.iter().any(|named| named == *script)) {
        if screens.iter().any(|(found, _, _)| *found == script) {
            continue;
        }
        match ythq_text(&archives, script) {
            Ok(Some(text)) => tablets_only.push((script.to_owned(), text)),
            Ok(None) => {
                made.problems.push(format!("{script}: not in YTHQ Tablet's files"));
                split = false;
            }
            Err(problem) => {
                made.problems.push(format!("{script}: {problem}"));
                split = false;
            }
        }
    }
    // The copies: WestPlay's tablet picture, a child of each script's window over the whole of it. European Truck
    // Simulator 2's alone: American Truck Simulator's tablets draw the factory screens' script itself.
    let mut copies: Vec<(String, Vec<u8>)> = Vec::new();
    if !together {
        for (script, text) in screens
            .iter()
            .map(|(script, _, text)| (*script, text.as_str()))
            .chain(tablets_only.iter().map(|(script, text)| (script.as_str(), text.as_str())))
        {
            match patch_ythq_tablet_script(text, &tablet, Some(&YTHQ_TABLET_SHOWN)) {
                Ok(patched) => copies.push((ythq_tablet_path(script), patched.into_bytes())),
                Err(problem) => {
                    made.problems.push(format!("{script}: {problem}"));
                    split = false;
                }
            }
        }
    }
    // Its tablets' models' screen materials: carried at paths of the mod's own, pointed at its tablets' own UI texture
    // (European Truck Simulator 2: the one its copies' scripts draw into) or kept as YTHQ has them (American Truck
    // Simulator: the trucks' own screens' texture, which its tablets' screens draw the factory screens' script into).
    let ui = if together { YTHQ_FACTORY_UI } else { YTHQ_TABLET_UI };
    let materials = if split || together {
        ythq_tablet_materials(&archives, &tablets.models, kind == crate::game::Kind::Ats, ui)
    } else {
        YthqMaterials::default()
    };
    made.problems.extend(materials.problems.iter().cloned());
    // Its factory screens' scripts: the truck screen's picture, and, American Truck Simulator's, WestPlay's tablet
    // picture too (on the part its tablets' screens show), or, European Truck Simulator 2's tablets' definitions not
    // pointed at copies (as before them), WestPlay's tablet picture over the whole window, which its tablets draw.
    for (script, layout, text) in &screens {
        let Some(index) = screen_layouts().iter().position(|found| found.id == *layout) else { continue };
        let texture = texture_name(&screen_layouts()[index]);
        let rows = ScreenLayout::texture_height(index);
        let shown = ythq_shown(kind, layout);
        let patched = patch_gps_script(text, &texture, shown, rows).and_then(|patched| {
            if together {
                patch_ythq_tablet_script(&patched, &tablet, Some(&YTHQ_TABLET_SHOWN))
            } else if split {
                Ok(patched)
            } else {
                patch_ythq_tablet_script(&patched, &tablet, None)
            }
        });
        match patched {
            Ok(patched) => {
                made.files.push(((*script).to_owned(), patched.into_bytes()));
                made.screens.push(script);
            }
            Err(problem) => made.problems.push(format!("{script}: {problem}")),
        }
    }
    if split && !together {
        made.tablets = copies.iter().map(|(path, _)| path.clone()).collect();
        made.tablets.sort();
        made.files.extend(copies);
        // Its tablets' definitions: each its `ui_path` the script's copy, and, where its model's screen material is
        // the mod's too, its `ui_drawable_texture_path` its tablets' own UI texture.
        let (factory, our_ui) = (format!("/{YTHQ_FACTORY_UI}"), format!("/{YTHQ_TABLET_UI}"));
        let mut definitions = Vec::with_capacity(tablets.definitions.len());
        for (path, text) in tablets.definitions {
            let model = ythq_model(&text).map(|model| model.trim_start_matches('/').to_owned());
            let ours = model.as_ref().is_some_and(|model| materials.models.contains(model));
            match ythq_texture_path(&text) {
                // Already the tablets' own (its tablets of the old trucks): as YTHQ has it.
                Some(texture) if texture == our_ui => definitions.push((path, text.into_bytes())),
                Some(texture) if texture == factory && ours => {
                    match repoint_quoted(&text, "ui_drawable_texture_path", &factory, &our_ui) {
                        Some(repointed) => definitions.push((path, repointed.into_bytes())),
                        None => {
                            made.problems.push(format!("{path}: its ui_drawable_texture_path not changed"));
                            definitions.push((path, text.into_bytes()));
                        }
                    }
                }
                // Its model's screen material not the mod's (not read as expected): the factory screen's picture then,
                // which is what the build had before its tablets' own texture.
                Some(texture) if texture == factory => {
                    made.problems.push(format!("{path}: its tablet's screen texture not its own ({texture})"));
                    definitions.push((path, text.into_bytes()));
                }
                _ => definitions.push((path, text.into_bytes())),
            }
        }
        made.accessories = definitions.iter().map(|(path, _)| path.clone()).collect();
        made.accessories.sort();
        made.files.extend(definitions);
    }
    if split || together {
        made.models = materials.models.iter().cloned().collect();
        made.materials = materials.screens;
        made.materials.sort();
        made.files.extend(materials.files);
    }
    made
}

/// Whether `path` is a screen script of YTHQ Tablet's (the game `kind`'s mod) that its tablets draw: one of its own
/// screens' ([`ythq_screens`], which its factory screens load) or one of [`YTHQ_TABLETS`].
fn ythq_script(kind: crate::game::Kind, path: &str) -> bool {
    ythq_screens(kind).iter().any(|(script, _)| *script == path) || YTHQ_TABLETS.contains(&path)
}

/// Where the mod carries the tablet-only copy of YTHQ Tablet's screen script `script`.
fn ythq_tablet_path(script: &str) -> String {
    format!("{YTHQ_TABLET_DIR}/{}", script.rsplit('/').next().unwrap_or(script))
}

/// YTHQ Tablet's tablets' definitions, as a build takes them ([`YTHQ_VEHICLES`]): those whose `ui_path` names one of
/// its screen scripts ([`ythq_script`]), by their own paths in the game, each `ui_path` changed to that script's copy
/// ([`YTHQ_TABLET_DIR`], other fields as YTHQ has them: `ui_drawable_texture_path`, `ui_drawable_size`, the models),
/// the scripts they named, the models they name, and why any wasn't as expected (its tablets then kept on the factory
/// screens' scripts).
#[derive(Default)]
struct YthqTablets {
    /// Each definition at its own path in the game, its `ui_path` the tablet copy's.
    definitions: Vec<(String, String)>,
    /// The scripts those definitions name.
    named: Vec<String>,
    /// The models those definitions name (`interior_model`), each once, sorted.
    models: Vec<String>,
    /// Why a definition wasn't as expected.
    problems: Vec<String>,
}

fn ythq_accessories(archives: &[Archive], kind: crate::game::Kind) -> YthqTablets {
    let mut made = YthqTablets::default();
    for root in YTHQ_VEHICLES {
        for vehicle in ythq_list(archives, root).1 {
            let accessories = format!("{root}{vehicle}/accessory/");
            for slot in ythq_list(archives, &accessories).1 {
                let dir = format!("{accessories}{slot}/");
                for name in ythq_list(archives, &dir).0.iter().filter(|name| name.ends_with(".sii")) {
                    let path = format!("{dir}{name}");
                    let text = match ythq_text(archives, &path) {
                        Ok(Some(text)) => text,
                        Ok(None) => {
                            made.problems.push(format!("{path}: not in YTHQ Tablet's files"));
                            continue;
                        }
                        Err(problem) => {
                            made.problems.push(format!("{path}: {problem}"));
                            continue;
                        }
                    };
                    let ui_paths = match ythq_ui_paths(&text) {
                        Ok(ui_paths) => ui_paths,
                        Err(problem) => {
                            made.problems.push(format!("{path}: {problem}"));
                            continue;
                        }
                    };
                    // Its own tablets': the ones of its screens. Any other accessory (YTHQ's, a mod's) is left as it
                    // is.
                    let ours: Vec<&String> = ui_paths
                        .iter()
                        .filter(|ui_path| ythq_script(kind, ui_path.trim_start_matches('/')))
                        .collect();
                    if ours.is_empty() {
                        continue;
                    }
                    let mut repointed = text;
                    let mut changed = true;
                    for ui_path in ours {
                        let script = ui_path.trim_start_matches('/');
                        let to = format!("/{}", ythq_tablet_path(script));
                        match repoint_ui_path(&repointed, ui_path, &to) {
                            Some(next) => {
                                repointed = next;
                                made.named.push(script.to_owned());
                            }
                            None => {
                                made.problems.push(format!("{path}: its ui_path {ui_path} not changed"));
                                changed = false;
                                break;
                            }
                        }
                    }
                    if changed {
                        if let Some(model) = ythq_model(&repointed)
                            && !made.models.contains(&model)
                        {
                            made.models.push(model);
                        }
                        made.definitions.push((path, repointed));
                    }
                }
            }
        }
    }
    made.models.sort();
    made
}

/// A tablet's definition's model (`interior_model: "/vehicle/…pmd"`, the path in the quotes), if it names one.
fn ythq_model(text: &str) -> Option<String> {
    ythq_quoted(text, "interior_model")
}

/// A tablet's definition's `ui_drawable_texture_path` (the texture object its screen draws its UI into), if it has
/// one.
fn ythq_texture_path(text: &str) -> Option<String> {
    ythq_quoted(text, "ui_drawable_texture_path")
}

/// A definition's quoted field `key`'s value (`key: "/some/path"`), if it has one.
fn ythq_quoted(text: &str, key: &str) -> Option<String> {
    let mut found = None;
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix(key) else { continue };
        let Some(rest) = rest.trim_start().strip_prefix(':') else { continue };
        found = rest.trim().strip_prefix('"').and_then(|value| value.strip_suffix('"')).map(str::to_owned);
    }
    found
}

/// YTHQ Tablet's tablet models' screen materials, as a build takes them: each model the tablets' definitions name, read
/// from its archive, its materials read, and those sampling its screens' texture object (the trucks' own screens',
/// [`YTHQ_FACTORY_UI`], or its tablets' own, [`YTHQ_TABLET_UI`]) carried with their screens sampling `ui`: its tablets'
/// own UI texture, so a tablet's screen shows its own script's UI (`ui_drawable_texture_path` its own too) rather than
/// the factory screen's picture, or, American Truck Simulator's, the trucks' own screens' texture as YTHQ has it.
/// `isolate`: each carried at a path of the mod's own ([`ythq_tablet_material_path`]), each model's descriptor changed
/// to those paths (the same length, in place), as American Truck Simulator needs: its YTHQ Tablet's screen materials are
/// the Ford cars' of `ats-ythq-ford` too, whose screens sample WestPlay's tablet texture at YTHQ's own paths, and are to
/// keep doing so. Without, at YTHQ's own paths (ETS2, whose YTHQ Tablet has those materials to itself). The models whose
/// screens show that texture; and why one couldn't be, if one couldn't.
#[derive(Default)]
struct YthqMaterials {
    /// What the mod carries of its, by path in the game: its tablets' models' screen materials, and, isolated, the
    /// models with those paths in them.
    files: Vec<(String, Vec<u8>)>,
    /// The screen materials it carries copies of, by YTHQ's own paths in the game.
    screens: Vec<String>,
    /// The models whose screens show its screens' texture, by path without its leading slash.
    models: std::collections::BTreeSet<String>,
    /// Why a model's screen couldn't be pointed at it.
    problems: Vec<String>,
}

fn ythq_tablet_materials(archives: &[Archive], models: &[String], isolate: bool, ui: &str) -> YthqMaterials {
    let mut made = YthqMaterials::default();
    let (factory, our_ui) = (format!("/{YTHQ_FACTORY_UI}"), format!("/{ui}"));
    // Its tablets' own screen texture is one of its files; the trucks' own screens' is the game's (YTHQ's definitions
    // name it, its archive needn't have it).
    if ui == YTHQ_TABLET_UI && ythq_bytes(archives, YTHQ_TABLET_UI).ok().flatten().is_none() {
        made.problems.push(format!("{YTHQ_TABLET_UI}: not in YTHQ Tablet's files (its tablets' own screen texture)"));
        return made;
    }
    // Each material read once: whether the model samples its screens' texture through it, and where the mod carries it
    // (its own path of YTHQ's). Its problems are said once.
    let mut read: std::collections::BTreeMap<String, (bool, Option<String>)> = std::collections::BTreeMap::new();
    for model in models {
        let path = model.trim_start_matches('/');
        let pmd = match ythq_bytes(archives, path) {
            Ok(Some(pmd)) => pmd,
            Ok(None) => {
                made.problems.push(format!("{path}: not in YTHQ Tablet's files"));
                continue;
            }
            Err(problem) => {
                made.problems.push(format!("{path}: {problem}"));
                continue;
            }
        };
        let mut shows = false;
        let mut changed = false;
        let mut patched = pmd.clone();
        for material in pmd_materials(&pmd) {
            let material = material.trim_start_matches('/').to_owned();
            let (screen, carried) = read
                .entry(material.clone())
                .or_insert_with(|| {
                    let text = match ythq_text(archives, &material) {
                        Ok(Some(text)) => text,
                        Ok(None) => {
                            made.problems.push(format!("{material}: not in YTHQ Tablet's files"));
                            return (false, None);
                        }
                        Err(problem) => {
                            made.problems.push(format!("{material}: {problem}"));
                            return (false, None);
                        }
                    };
                    let ours = text.contains(&format!("\"{our_ui}\""));
                    if !ours && !text.contains(&format!("\"{factory}\"")) {
                        return (false, None);
                    }
                    // Already its tablets' own at YTHQ's own path (its tablets of the old trucks): the model samples it.
                    if ours && !isolate {
                        return (true, None);
                    }
                    let carried = if isolate { ythq_tablet_material_path(&material) } else { material.clone() };
                    let text = if ours {
                        text
                    } else {
                        text.replace(&format!("\"{factory}\""), &format!("\"{our_ui}\""))
                    };
                    made.files.push((carried.clone(), text.into_bytes()));
                    made.screens.push(material.clone());
                    (true, Some(carried))
                })
                .clone();
            if screen {
                shows = true;
            }
            if let Some(carried) = carried
                && isolate
                && replace_path(&mut patched, &material, &carried)
            {
                changed = true;
            }
        }
        if shows {
            made.models.insert(path.to_owned());
            if changed {
                made.files.push((path.to_owned(), patched));
            }
        } else {
            made.problems.push(format!("{path}: no screen material on the trucks' own screens' texture"));
        }
    }
    made.files.sort_by(|a, b| a.0.cmp(&b.0));
    made.screens.sort();
    made.screens.dedup();
    made
}

/// Where the mod carries its own copy of YTHQ Tablet's screen material `material` (a path in the game, without its
/// leading slash): a path of its own, the same length (so the models' descriptors are changed in place), its name the
/// hash of YTHQ's (as the game's own materials' are, in the folder their first two digits would be in). Deterministic: a
/// build's copy is at the same path. A path not laid out as a material's (`/automat/xx/<16 hex>.mat`) is left as it is.
fn ythq_tablet_material_path(material: &str) -> String {
    let name = material.rsplit('/').next().unwrap_or(material);
    let Some(stem) = name.strip_suffix(".mat").filter(|stem| stem.len() == 16) else { return material.to_owned() };
    if !stem.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return material.to_owned();
    }
    let mut hash =
        westplay_mapdata::cityhash::city_hash64(format!("{material}\u{0}westplay.ythq.tablet").as_bytes());
    if format!("{hash:016x}") == stem {
        hash ^= 1;
    }
    let hash = format!("{hash:016x}");
    // In the folder the material's own name would be in, as the game's own are.
    if material.starts_with("automat/") {
        format!("automat/{}/{hash}.mat", &hash[..2])
    } else {
        format!("{}{hash}.mat", &material[..material.len() - name.len()])
    }
}

/// `data` with its paths `from` changed to `to` (the same length), the rest of it as it is; whether any was.
fn replace_path(data: &mut [u8], from: &str, to: &str) -> bool {
    let (from, to) = (from.as_bytes(), to.as_bytes());
    if from.is_empty() || from.len() != to.len() {
        return false;
    }
    let mut changed = false;
    let mut at = 0;
    while let Some(found) = data.get(at..).and_then(|rest| rest.windows(from.len()).position(|window| window == from))
    {
        let start = at + found;
        data[start..start + to.len()].copy_from_slice(to);
        changed = true;
        at = start + to.len();
    }
    changed
}

/// A tablet model's (`.pmd`'s) materials: the `/automat/xx/….mat` paths it names, in the order it names them.
fn pmd_materials(data: &[u8]) -> Vec<String> {
    const PREFIX: &[u8] = b"/automat/";
    const HEX: &[u8] = b"0123456789abcdef";
    let mut found = Vec::new();
    let mut at = 0;
    while at + PREFIX.len() < data.len() {
        let Some(start) = data[at..].windows(PREFIX.len()).position(|window| window == PREFIX) else { break };
        let start = at + start;
        let mut end = start + PREFIX.len();
        let name_len = |data: &[u8], at: usize, len: usize| -> bool {
            data.len() >= at + len && data[at..at + len].iter().all(|byte| HEX.contains(byte))
        };
        if name_len(data, end, 2) && data.get(end + 2) == Some(&b'/') && name_len(data, end + 3, 16) {
            end += 3 + 16;
            if data.get(end..end + 4) == Some(b".mat") {
                end += 4;
                if let Ok(path) = std::str::from_utf8(&data[start..end]) {
                    found.push(path.to_owned());
                }
                at = end;
                continue;
            }
        }
        at = start + PREFIX.len();
    }
    found
}

/// A definition's `ui_path` values (`ui_path: "/ui/dashboard/…"`, the path in the quotes), in the order they're
/// written; not as expected if one isn't a quoted path on its own line.
fn ythq_ui_paths(text: &str) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("ui_path") else { continue };
        if rest.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let Some(rest) = rest.trim_start().strip_prefix(':') else {
            return Err(format!("ui_path not a field: {}", trimmed.trim_end()));
        };
        let value = rest.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or_else(|| format!("ui_path {value} not a quoted path"))?;
        found.push(value.to_owned());
    }
    Ok(found)
}

/// `text` with its `ui_path` `from` changed to `to`, the rest of the definition as it is; none if it has no such
/// `ui_path`.
fn repoint_ui_path(text: &str, from: &str, to: &str) -> Option<String> {
    repoint_quoted(text, "ui_path", from, to)
}

/// `text` with its field `key`'s quoted path `from` changed to `to` (on its first line that has it), the rest of the
/// definition as it is; none if it has no line with that `key` and path.
fn repoint_quoted(text: &str, key: &str, from: &str, to: &str) -> Option<String> {
    let (from, to) = (format!("\"{from}\""), format!("\"{to}\""));
    let mut out = String::with_capacity(text.len() + to.len());
    let mut changed = false;
    for line in text.split_inclusive('\n') {
        let ours = !changed && line.trim_start().starts_with(key) && line.contains(&from);
        if ours {
            out.push_str(&line.replacen(&from, &to, 1));
            changed = true;
        } else {
            out.push_str(line);
        }
    }
    changed.then_some(out)
}

/// YTHQ Tablet's file `path`'s bytes, as its archive has them; none if its archive hasn't it, and why not if it isn't
/// read.
fn ythq_bytes(archives: &[Archive], path: &str) -> Result<Option<Vec<u8>>, String> {
    for archive in archives {
        match archive.read(path) {
            Ok(Some(bytes)) => return Ok(Some(bytes)),
            Ok(None) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(None)
}

/// YTHQ Tablet's file `path`'s text, unscrambled (as its definitions are scrambled: `save::text`); none if its archive
/// hasn't it, and why not if it isn't read.
fn ythq_text(archives: &[Archive], path: &str) -> Result<Option<String>, String> {
    let Some(bytes) = ythq_bytes(archives, path)? else { return Ok(None) };
    let text = save::text(&bytes).map_err(|error| error.to_string())?;
    String::from_utf8(text.into_owned()).map(Some).map_err(|_| "not UTF-8 text".to_owned())
}

/// A folder in YTHQ Tablet's archives: its files' names and its subfolders' (without their slashes), all of its
/// archives' together.
fn ythq_list(archives: &[Archive], dir: &str) -> (Vec<String>, Vec<String>) {
    let (mut files, mut dirs) = (Vec::new(), Vec::new());
    for archive in archives {
        if let Ok(Some(listing)) = archive.list(dir) {
            files.extend(listing.files);
            dirs.extend(listing.dirs);
        }
    }
    files.sort();
    files.dedup();
    dirs.sort();
    dirs.dedup();
    (files, dirs)
}

/// Whether the mod at `built` (WestPlay's `.scs`) has YTHQ Tablet's screens as [`ythq_screen_files`] makes them from
/// the archive in `workshop` now: not so once YTHQ Tablet is installed after the mod was made, or updated (its screen
/// scripts are the trucks' factory screens then, and the mod's old copies are YTHQ's other version).
pub fn ythq_screens_current(kind: crate::game::Kind, workshop: Option<&Path>, built: &Path) -> bool {
    let made = ythq_screen_files(workshop, kind);
    if made.files.is_empty() {
        return true;
    }
    let Ok(archive) = Archive::open(built) else { return false };
    let ours = GameFiles::new(vec![Mount::Archive(archive)]);
    made.files.iter().all(|(path, bytes)| ours.read(path).ok().flatten().as_ref() == Some(bytes))
}

/// The mod built from `game` (the installed game, mounted), as the bytes of its `.scs` file; the tablet at the
/// game's trucks' slots for it and at `mod_trucks` (the installed mods': `tablet::mod_trucks`); the cab's mark in the
/// dashboard scripts of `cab_marks` (`cab_mark::for_tablet`). YTHQ Tablet's own screen scripts are patched and carried
/// too (WestPlay's truck screen's picture and its tablet's), from its archive in `workshop` (the game's Workshop
/// content folder: none, nothing of its to take over).
/// Nothing of a mod's (owner 2026-11-16: WestPlay works on the vehicles it supports alone, the tablet in any).
pub fn build(
    game: &GameFiles,
    mod_trucks: &[crate::tablet::Spot],
    cab_marks: &cab_mark::Marks,
    kind: crate::game::Kind,
    workshop: Option<&Path>,
) -> Result<(Vec<u8>, Built), BuildError> {
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("manifest.sii".into(), manifest().into_bytes()),
        ("description.txt".into(), b"WestPlay screens. Requires the WestPlay app and plugin.\n".to_vec()),
    ];
    let mut built = Built::default();
    for (index, layout) in screen_layouts().iter().enumerate() {
        let texture = texture_name(layout);
        let rows = ScreenLayout::texture_height(index);
        files.push((format!("{SCREEN_DIR}/{texture}.mat"), material(&texture).into_bytes()));
        files.push((format!("{SCREEN_DIR}/{texture}.tobj"), tobj(&format!("/{SCREEN_DIR}/{texture}.dds"))));
        files.push((format!("{SCREEN_DIR}/{texture}.dds"), transparent_dds(SCREEN_WIDTH, rows)));
        // WestPlay's tablet: its model shows the texture (no script draws it), whole or missing.
        if layout.id == TABLET {
            let screen = format!("/{SCREEN_DIR}/{texture}.tobj");
            let trucks = crate::tablet::trucks(game).map(|own| {
                let all: std::collections::BTreeSet<crate::tablet::Spot> =
                    own.into_iter().chain(mod_trucks.iter().cloned()).collect();
                all.into_iter().collect::<Vec<_>>()
            });
            // Each truck's hanging point's pitch, from its interiors in the game (a mod truck's aren't there: none).
            let made = trucks.and_then(|trucks| {
                let pitches = crate::tablet::hanging_pitches(game, &trucks);
                crate::tablet::files(&trucks, &pitches, &screen, ScreenLayout::texture_height(index))
            });
            match made {
                Ok(tablet) => {
                    files.extend(tablet);
                    built.patched.push(layout.id);
                }
                Err(_) => built.missing.push(layout.id),
            }
            continue;
        }
        // A screen of WestPlay's own making: whole, or missing.
        if let Some(own) = OWN_SCREENS.iter().find(|own| own.layout == layout.id) {
            match layout.gps_scripts {
                [script] => match own_screen_files(game, own, script, &texture, rows) {
                    Ok(own_files) => {
                        files.extend(own_files);
                        built.patched.push(layout.id);
                    }
                    Err(_) => built.missing.push(layout.id),
                },
                _ => built.missing.push(layout.id),
            }
            continue;
        }
        // Each of its scripts the game has (a layout whose scripts it has none of is missing).
        let scripts = game_screen_scripts(game, index)?;
        if scripts.is_empty() {
            built.missing.push(layout.id);
        } else {
            files.extend(scripts);
            built.patched.push(layout.id);
        }
    }
    // The cab's mark: its texture, fully transparent, and the dashboard scripts with it, listed.
    let mark = cab_mark::TEXTURE;
    files.push((format!("{SCREEN_DIR}/{mark}.mat"), material(mark).into_bytes()));
    files.push((format!("{SCREEN_DIR}/{mark}.tobj"), tobj(&format!("/{SCREEN_DIR}/{mark}.dds"))));
    files.push((format!("{SCREEN_DIR}/{mark}.dds"), transparent_dds(CAB_MARK_WIDTH, CAB_MARK_HEIGHT)));
    // The parking camera's mark (in the GPS scripts with a camera): its texture, fully transparent.
    let parking = PARKING_MARK;
    files.push((format!("{SCREEN_DIR}/{parking}.mat"), material(parking).into_bytes()));
    files.push((format!("{SCREEN_DIR}/{parking}.tobj"), tobj(&format!("/{SCREEN_DIR}/{parking}.dds"))));
    files.push((format!("{SCREEN_DIR}/{parking}.dds"), transparent_dds(PARKING_MARK_WIDTH, PARKING_MARK_HEIGHT)));
    files.push((cab_mark::LIST.to_owned(), cab_marks.list().into_bytes()));
    files.extend(cab_marks.scripts.iter().cloned());
    built.cab_marks = cab_marks.scripts.len();
    // The 7" screen at a size it shows sharp, if its interiors are as read (else the game's size stays, and
    // the build says why).
    match sharp_screen_files(game) {
        Ok(sharp) => files.extend(sharp),
        Err(why) => built.not_sharp = Some(why),
    }
    for path in SHARP_SCREENS {
        match sharp_screens_file(game, path) {
            Ok(Some(sharp)) => files.push(((*path).to_owned(), sharp)),
            Ok(None) => {}
            Err(why) => {
                built.not_sharp.get_or_insert(why);
            }
        }
    }
    // The coaches' screens drawn sharp, where their screens are WestPlay's.
    let (sized, why) = bus_screens_files(game, &built.patched);
    files.extend(sized);
    if let Some(why) = why {
        built.not_sharp.get_or_insert(why);
    }
    // What the add-ons bring, in place of the mod's own of the same path (their scripts say what WestPlay's screen
    // shows where they cover it).
    let extras = addons(kind);
    let covered: std::collections::BTreeSet<String> = extras.iter().map(|(path, _)| path.clone()).collect();
    files.retain(|(path, _)| !covered.contains(path));
    files.extend(extras);
    // YTHQ Tablet's own screens for the trucks' factory screens, patched from its archive and over its (and the
    // add-ons') copies: installed, the game loads them as the factory screens' `gps_path` and never the game's
    // scripts the mod patches (`ythq_screens`). Its tablets' definitions are pointed at its scripts' copies
    // (`YTHQ_TABLET_DIR`) and at its tablets' own UI texture ([`YTHQ_TABLET_UI`], their models' too), so they show
    // the tablet's picture alone. Its archive not read as expected leaves the mod as it was.
    let ythq = ythq_screen_files(workshop, kind);
    let covered: std::collections::BTreeSet<&str> = ythq.files.iter().map(|(path, _)| path.as_str()).collect();
    files.retain(|(path, _)| !covered.contains(path.as_str()));
    built.ythq = ythq.screens;
    built.ythq_tablets = ythq.tablets;
    built.ythq_accessories = ythq.accessories;
    built.ythq_tablet_materials = ythq.materials;
    built.ythq_problem = (!ythq.problems.is_empty()).then(|| ythq.problems.join("; "));
    files.extend(ythq.files);
    // Its tablets' models whose screens' materials the mod carried at paths of its own (its screens' texture again):
    // the add-ons' models of them are left out, YTHQ's own used, whose screens' faces are laid out for that texture.
    let models: std::collections::BTreeSet<String> =
        ythq.models.iter().map(|model| model.replace(".pmd", ".pmg")).collect();
    files.retain(|(path, _)| !models.contains(path));
    // Its tablets draw copies of its screen scripts ([`YTHQ_TABLETS`]' included): the add-ons' copies of those two at
    // their own paths (WestPlay's tablet picture over YTHQ's UI) are left out, so the factory screens that can still
    // load them (the Volvo 9700's, the only coach's whose screen isn't WestPlay's) keep YTHQ's own UI rather than
    // WestPlay's tablet picture stretched over it. (Its tablets' definitions not read as expected: the add-ons' kept,
    // the tablets drawing those scripts as before.)
    if !built.ythq_tablets.is_empty() {
        files.retain(|(path, _)| !YTHQ_TABLETS.contains(&path.as_str()));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((zip(&files), built))
}

/// The layout at `index`'s GPS scripts as the mod has them, of a screen the game draws itself (not the tablet, a mod
/// vehicle's or one of WestPlay's own making): each script the game has, patched; a window screen's, WestPlay's own
/// window where the game has none. None: the game has none of them (a DLC not installed).
fn game_screen_scripts(game: &GameFiles, index: usize) -> Result<Vec<(String, Vec<u8>)>, BuildError> {
    let layout = &screen_layouts()[index];
    let (texture, rows) = (texture_name(layout), ScreenLayout::texture_height(index));
    let window = WINDOW_SCREENS.iter().find(|screen| screen.layout == layout.id);
    let mut files = Vec::new();
    for &script in layout.gps_scripts {
        let original = game.read(script).map_err(|error| BuildError::Read(error.to_string()))?;
        let text = match (original, window) {
            (Some(original), _) => String::from_utf8(original)
                .map_err(|_| BuildError::Script { script, problem: "not UTF-8 text".into() })?,
            (None, Some(_)) => own_window(),
            (None, None) => continue,
        };
        let shown = SHOWN_PARTS.iter().find(|(id, _)| *id == layout.id).map(|(_, part)| part);
        let patched = match window {
            Some(window) => patch_window_script(&text, &texture, &window.shown, rows),
            None => patch_gps_script(&text, &texture, shown, rows),
        }
        .map_err(|problem| BuildError::Script { script, problem })?;
        files.push((script.to_owned(), patched.into_bytes()));
    }
    Ok(files)
}

/// Whether the layout at `index` is a screen the game draws itself ([`game_screen_scripts`]).
fn game_screen(index: usize) -> bool {
    let id = screen_layouts()[index].id;
    id != TABLET && !OWN_SCREENS.iter().any(|own| own.layout == id)
}

/// The coaches' screens files ([`BUS_SCREENS`]) made [`BUS_SIZE`], of those whose script's layout is among `patched`
/// (WestPlay's screens in the mod); and why one couldn't be made, if one couldn't.
fn bus_screens_files(game: &GameFiles, patched: &[&str]) -> (Vec<(String, Vec<u8>)>, Option<String>) {
    let (mut files, mut why) = (Vec::new(), None);
    for &(path, script) in BUS_SCREENS {
        let ours =
            screen_layouts().iter().any(|layout| layout.gps_scripts.contains(&script) && patched.contains(&layout.id));
        match sized_screens_file(game, path, script, BUS_SIZE) {
            Ok(Some(sized)) if ours => files.push((path.to_owned(), sized)),
            Ok(_) => {}
            Err(problem) => {
                why.get_or_insert(problem);
            }
        }
    }
    (files, why)
}

/// Whether the mod built at `built` (WestPlay's `.scs`) has the screens the game draws itself as `game` (the installed
/// game) would make them now: each of their scripts patched from the game's own, the same, none the game no longer
/// has, and the coaches' screens files with them. Not so once a DLC with a screen of WestPlay's is installed after
/// the mod was made (a player's Coaches DLC, 2026-11-15: the coaches showed the game's navigation), or a game update
/// changes a script.
pub fn game_screens_current(game: &GameFiles, built: &Path) -> bool {
    let Ok(archive) = Archive::open(built) else { return false };
    let ours = GameFiles::new(vec![Mount::Archive(archive)]);
    let same = |path: &str, bytes: Option<&Vec<u8>>| ours.read(path).ok().flatten().as_ref() == bytes;
    let mut patched = Vec::new();
    for index in (0..screen_layouts().len()).filter(|&index| game_screen(index)) {
        let layout = &screen_layouts()[index];
        let Ok(files) = game_screen_scripts(game, index) else { return false };
        let made = |script: &str| files.iter().find(|(path, _)| path == script).map(|(_, bytes)| bytes);
        if !layout.gps_scripts.iter().all(|&script| same(script, made(script))) {
            return false;
        }
        if !files.is_empty() {
            patched.push(layout.id);
        }
    }
    let (sized, _) = bus_screens_files(game, &patched);
    BUS_SCREENS.iter().all(|&(path, _)| same(path, sized.iter().find(|(at, _)| at == path).map(|(_, bytes)| bytes)))
}

/// Writes the mod into `mod_dir` (WestPlay's own folder), replacing the one there whole: written beside it
/// first, then moved over it, so the plugin never finds half a file.
/// A step that failed names itself and its path (`writing C:\…\WestPlay\westplay.scs.partial: …`), so a
/// player's report says which.
pub fn install(bytes: &[u8], mod_dir: &Path, kind: Kind) -> std::io::Result<PathBuf> {
    let step = |what: &str, path: &Path| {
        let what = format!("{what} {}", path.display());
        move |error: std::io::Error| std::io::Error::new(error.kind(), format!("{what}: {error}"))
    };
    std::fs::create_dir_all(mod_dir).map_err(step("making the folder", mod_dir))?;
    let path = mod_dir.join(file_name(kind));
    let partial = mod_dir.join(format!("{}.partial", file_name(kind)));
    std::fs::write(&partial, bytes).map_err(step("writing", &partial))?;
    std::fs::rename(&partial, &path).map_err(step("moving it over", &path))?;
    Ok(path)
}

/// A layout's screen texture's name: `screen_daf_2021`.
pub fn texture_name(layout: &ScreenLayout) -> String {
    format!("screen_{}", layout.id.replace('-', "_"))
}

/// The mod's manifest. Optional in convoys (`mp_mod_optional`, as Workshop mods mark it): it changes only the
/// player's own truck screen, so players with and without it join each other's convoys.
fn manifest() -> String {
    format!(
        "SiiNunit\n{{\nmod_package : .package_name\n{{\n\tpackage_version: \"{}\"\n\tdisplay_name: \"WestPlay\"\n\tauthor: \"Western\"\n\tcategory[]: \"ui\"\n\tdescription_file: \"description.txt\"\n\tmp_mod_optional: true\n}}\n}}\n",
        env!("CARGO_PKG_VERSION")
    )
}

/// A UI material showing `texture`'s texture object, beside it (the screens', and the tablet's shop icon).
pub(crate) fn material(texture: &str) -> String {
    format!(
        "effect : \"ui.rfx\" {{\n\ttexture : \"texture\" {{\n\t\tsource : \"{texture}.tobj\"\n\t\tu_address : clamp\n\t\tv_address : clamp\n\t\tmip_filter : none\n\t}}\n}}\n"
    )
}

/// A texture object pointing at `dds_path`: the header of a stock UI texture object (`/material/ui/white.tobj`),
/// then the path.
pub(crate) fn tobj(dds_path: &str) -> Vec<u8> {
    const HEADER: [u8; 40] = [
        0x01, 0x0a, 0xb1, 0x70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0, 0, 0, 0x02, 0, 0x01, 0x01,
        0x02, 0, 0x02, 0x02, 0, 0, 0, 0, 0, 0x01, 0, 0,
    ];
    let mut bytes = HEADER.to_vec();
    bytes.extend((dds_path.len() as u32).to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(dds_path.as_bytes());
    bytes
}

/// A fully transparent `width` x `height` texture: a DDS with a DX10 header, B8G8R8A8_UNORM_SRGB (91), the
/// format the plugin copies WestPlay's frames into.
fn transparent_dds(width: u32, height: u32) -> Vec<u8> {
    screen_dds(width, height, &vec![0; (width * height * 4) as usize])
}

/// A `width` x `height` texture of `pixels` (blue, green, red, alpha each, row by row), laid out as the screens'
/// ([`transparent_dds`]).
pub(crate) fn screen_dds(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let mut header = [0u8; 148];
    header[0..4].copy_from_slice(b"DDS ");
    for (at, value) in [(4, 124u32), (8, 0x1007), (12, height), (16, width), (76, 32), (80, 0x4), (108, 0x1000)] {
        header[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    header[84..88].copy_from_slice(b"DX10");
    for (at, value) in [(128, 91u32), (132, 3), (136, 0), (140, 1), (144, 0)] {
        header[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut bytes = header.to_vec();
    bytes.extend(pixels);
    bytes
}

/// A top-level `ui::kind : name {` block of a UI script: its kind and name, and its lines (header to brace).
struct Block<'a> {
    kind: &'a str,
    name: &'a str,
    start: usize,
    end: usize,
}

fn blocks<'a>(lines: &[&'a str]) -> Vec<Block<'a>> {
    let mut blocks = Vec::new();
    let mut current: Option<(&str, &str, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        if current.is_none() {
            current = block_header(line).map(|(kind, name)| (kind, name, index));
        } else if line.trim() == "}" {
            let (kind, name, start) = current.take().unwrap();
            blocks.push(Block { kind, name, start, end: index });
        }
    }
    blocks
}

/// `ui::group : _nameless.0abc {` → (`ui::group`, `_nameless.0abc`), indented as a mod's script may write it (YTHQ
/// Tablet's scripts indent every block, the game's start at the line's beginning).
fn block_header(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim().strip_suffix('{')?.trim_end();
    let (kind, name) = rest.split_once(" : ")?;
    let kind_ok = kind
        .strip_prefix("ui::")
        .is_some_and(|word| !word.is_empty() && word.chars().all(|c| c.is_alphanumeric() || c == '_'));
    (kind_ok && !name.is_empty() && !name.contains(char::is_whitespace)).then_some((kind, name))
}

/// A field's value in a block (`id: 100` → `100`).
fn field<'a>(lines: &[&'a str], block: &Block, name: &str) -> Option<&'a str> {
    lines[block.start + 1..block.end].iter().find_map(|line| {
        let (key, value) = line.trim().split_once(':').unwrap_or((line.trim(), ""));
        (key == name).then(|| value.trim())
    })
}

/// Adds the WestPlay screen image `texture` as the top layer of the GPS script's "screen on" group (id 100):
/// a child of the group, over the whole of it, or over the part of it the display shows (`shown`). The texture is
/// `rows` tall (`ScreenLayout::texture_height`): the frame's [`SCREEN_HEIGHT`] rows go on that part, the rows beyond
/// them (the layout's mark, repeating the frame's bottom row) below it, off the screen; stretched over the part
/// whole, they squashed the frame up to 1.6% and smeared its bottom row (its black round corners) along the
/// screen's bottom edge (2026-11-03). The rest of the script stays as it is.
pub fn patch_gps_script(text: &str, texture: &str, shown: Option<&ShownPart>, rows: u32) -> Result<String, String> {
    let lifted = lift_parking_camera(text)?;
    let text = lifted.as_str();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(newline).collect();
    let blocks = blocks(&lines);
    let group = blocks
        .iter()
        .find(|block| block.kind == "ui::group" && field(&lines, block, "id") == Some(SCREEN_ON_GROUP))
        .ok_or("no group with id 100")?;
    let count: usize =
        field(&lines, group, "my_children").unwrap_or("0").parse().map_err(|_| "my_children isn't a count")?;
    let coord = |name| field(&lines, group, name).ok_or_else(|| format!("the group has no {name}"));
    let number = |text: &str| text.parse::<f64>().map_err(|_| format!("the group's coordinate {text}"));
    let (l, r, t, b) =
        (number(coord("coords_l")?)?, number(coord("coords_r")?)?, number(coord("coords_t")?)?, number(coord("coords_b")?)?);
    let whole = ShownPart { left: 0.0, right: 1.0, top: 0.0, bottom: 1.0 };
    let part = shown.unwrap_or(&whole);
    // Its coordinates go up the screen: the top is the larger. The frame's rows on the part, the texture's
    // reaching below it by its extra rows.
    let (top, bottom) = (t - (t - b) * part.top, t - (t - b) * part.bottom);
    let bottom = top - (top - bottom) * f64::from(rows) / f64::from(SCREEN_HEIGHT);
    let at = |value: f64| format!("{}", value.round() as i64);
    let (left, right, top, bottom) = (at(l + (r - l) * part.left), at(l + (r - l) * part.right), at(top), at(bottom));
    let element = "_nameless.westplay.screen";

    let mut body = Vec::new();
    for line in &lines[group.start + 1..group.end] {
        let trimmed = line.trim();
        let line = if trimmed.starts_with("my_children:") {
            format!(" my_children: {}", count + 1)
        } else {
            (*line).to_owned()
        };
        let after = if count == 0 {
            trimmed.starts_with("my_children:")
        } else {
            trimmed.starts_with(&format!("my_children[{}]:", count - 1))
        };
        body.push(line);
        if after {
            body.push(format!(" my_children[{count}]: {element}"));
        }
    }
    let screen = [
        String::new(),
        format!("ui::text : {element} {{"),
        format!(" text: \"<img src=/{SCREEN_DIR}/{texture}.mat xscale=stretch yscale=stretch>\""),
        format!(" coords_l: {left}"),
        format!(" coords_r: {right}"),
        format!(" coords_t: {top}"),
        format!(" coords_b: {bottom}"),
        " area_l: 1".into(),
        " area_r: 0".into(),
        " area_t: 0".into(),
        " area_b: 1".into(),
        " id: 0".into(),
        format!(" layer: {SCREEN_LAYER}"),
        " tab: -1".into(),
        " pointer: -1".into(),
        format!(" my_parent: {}", group.name),
        "}".into(),
    ];
    let last_end = blocks.last().map(|block| block.end).ok_or("no blocks")?;
    let mut patched: Vec<String> = lines[..=group.start].iter().map(|line| (*line).to_owned()).collect();
    patched.extend(body);
    patched.extend(lines[group.end..=last_end].iter().map(|line| (*line).to_owned()));
    patched.extend(screen);
    patched.extend(lines[last_end + 1..].iter().map(|line| (*line).to_owned()));
    let patched = patched.join(newline);
    // The parking camera's mark in its group, drawn with it: the screen under the camera then (no clicks to it).
    match parking_camera(&lines, &blocks) {
        Some(camera) => {
            add_to(&patched, camera.name, "_nameless.westplay.parking", PARKING_MARK, SCREEN_LAYER, mark_corner)
        }
        None => Ok(patched),
    }
}

/// YTHQ Tablet's script `text` with WestPlay's tablet picture added, as well as the truck screen's
/// ([`patch_gps_script`]): the `tablet` layout's texture (`texture`) as a second image, a child of the script's window
/// over the whole of it, or over the part of it its tablets' screens show (`shown`: American Truck Simulator's, whose
/// one script its tablets draw too, [`YTHQ_TABLET_SHOWN`]), and under the truck screen's ([`TABLET_LAYER`]). YTHQ
/// Tablet's screens are UI accessories whose `ui_path` names these same scripts
/// (`def/vehicle/truck/*/accessory/toyac/*.sii`, `ui_drawable_size` (512, 512), or American Truck Simulator's (1024,
/// 1024)): the plugin draws frames into one of the two textures, the app's "WestPlay on" setting picking it, and keeps
/// the other transparent, so the "WestPlay tablet" shows the tablet layout's picture there (the one the app lays out for
/// the tablet) and the "truck screen" its layout's, each on the screen's own.
///
/// The window's own coordinates: YTHQ's pictures fill the window, and so the tablet's is shown whole. (The bridge that
/// first overlaid it drew it 5.47% wider than the window and let the window's `clip_children` crop its sides, the
/// owner finding it a size too big: 2026-11-16, the add-ons' copies shrunk back to the window, [`YTHQ_INSET_X`].) The
/// rest of the script stays as it is.
fn patch_ythq_tablet_script(text: &str, texture: &str, shown: Option<&ShownPart>) -> Result<String, String> {
    if text.contains(TABLET_OVERLAY) {
        return Err("has the tablet's picture already".into());
    }
    match shown {
        None => add_to_window(text, TABLET_OVERLAY, texture, TABLET_LAYER, |coords| coords),
        // The part of the window the display shows, as the truck screen's picture goes on its own
        // ([`patch_gps_script`]): its bounds within the window's, the texture over those whole.
        Some(shown) => add_to_window(text, TABLET_OVERLAY, texture, TABLET_LAYER, |[left, right, top, bottom]| {
            let across = |share: f64| left + (right - left) * share;
            let down = |share: f64| top - (top - bottom) * share;
            [across(shown.left), across(shown.right), down(shown.top), down(shown.bottom)]
        }),
    }
}

/// A GPS script's parking camera: its group with id [`PARKING_CAMERA_GROUP`], if it has one.
fn parking_camera<'a, 'b>(lines: &[&'a str], blocks: &'b [Block<'a>]) -> Option<&'b Block<'a>> {
    blocks.iter().find(|block| block.kind == "ui::group" && field(lines, block, "id") == Some(PARKING_CAMERA_GROUP))
}

/// A mark's place in a block (left, right, top, bottom, from the block's own): its left and bottom corner, in from
/// both by [`MARK_SHARE`] of the block each way (its coordinates may go up the screen or down it).
fn mark_corner([left, right, top, bottom]: [f64; 4]) -> [f64; 4] {
    let side = |span: f64| (span.abs() / MARK_SHARE).ceil().max(1.0);
    let inward = if top >= bottom { 1.0 } else { -1.0 };
    [left, left + side(right - left), bottom + inward * side(top - bottom), bottom]
}

/// The parking camera of a GPS script (its group with id [`PARKING_CAMERA_GROUP`], and everything in it) a layer of
/// its own over WestPlay's screen ([`SCREEN_LAYER`]): each of its elements' layers past it by its own, so the
/// camera keeps its order (its background, its picture, its marks). The game draws by layers across the whole
/// script, so WestPlay's image (in the "screen on" group) was drawn over the camera, though the camera's group comes
/// after that group (owner 2026-11-13, reversing a coach: "WestPlay should be under it, not over"). A script with
/// no camera stays as it is.
fn lift_parking_camera(text: &str) -> Result<String, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(newline).collect();
    let blocks = blocks(&lines);
    let Some(camera) = parking_camera(&lines, &blocks) else {
        return Ok(text.to_owned());
    };
    // The group, and what's in it at any depth (by each block's parent).
    let mut inside = vec![camera.name];
    loop {
        let before = inside.len();
        for block in &blocks {
            if !inside.contains(&block.name)
                && field(&lines, block, "my_parent").is_some_and(|parent| inside.contains(&parent))
            {
                inside.push(block.name);
            }
        }
        if inside.len() == before {
            break;
        }
    }
    let mut lifted: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    for block in blocks.iter().filter(|block| inside.contains(&block.name)) {
        let at = (block.start + 1..block.end)
            .find(|&index| lines[index].trim().split_once(':').is_some_and(|(key, _)| key == "layer"))
            .ok_or_else(|| format!("the parking camera's {} has no layer", block.name))?;
        let value = lines[at].trim().split_once(':').map_or("", |(_, value)| value.trim());
        let layer: u32 =
            value.parse().map_err(|_| format!("the parking camera's {} has the layer {value}", block.name))?;
        let indent = &lines[at][..lines[at].len() - lines[at].trim_start().len()];
        lifted[at] = format!("{indent}layer: {}", SCREEN_LAYER + 1 + layer);
    }
    Ok(lifted.join(newline))
}

/// Adds the cab's mark (`texture`, `cab_mark`) to a dashboard script: a child of its window (its one block with no
/// parent), and of each group directly in the window that covers all of it, a square in the corner, fully
/// transparent. The rest of the script stays as it is.
///
/// The newer dashboards (the Volvo VNL 2025's, the Kenworth T680 2022's: a 1440 by 540 window) hold everything in
/// such groups, one display each, and a mark of a unit there was never drawn: in the VNL 2025, looking away from the
/// tablet was taken for leaving the cab, the music muffled (owner 2026-11-06). So the mark is some pixels on any
/// screen, 1/64 of the window each way ([`MARK_SHARE`]), and in each display as well as the window.
pub fn patch_dashboard_script(text: &str, texture: &str) -> Result<String, String> {
    if text.contains("_nameless.westplay.cab") {
        return Err("has the mark already".into());
    }
    let mut patched = add_to_window(text, "_nameless.westplay.cab", texture, SCREEN_LAYER, mark_corner)?;
    for (index, group) in full_window_groups(text)?.into_iter().enumerate() {
        patched = add_to(
            &patched,
            &group,
            &format!("_nameless.westplay.cab{index}"),
            texture,
            SCREEN_LAYER,
            mark_corner,
        )?;
    }
    Ok(patched)
}

/// How much of a dashboard's window the cab's mark covers each way: 1/64 (at least a unit).
const MARK_SHARE: f64 = 64.0;

/// The groups directly in a UI script's window (its one block with no parent) that cover all of it, by name.
fn full_window_groups(text: &str) -> Result<Vec<String>, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(newline).collect();
    let blocks = blocks(&lines);
    let roots: Vec<&Block> = blocks.iter().filter(|block| field(&lines, block, "my_parent") == Some("null")).collect();
    let [window] = roots[..] else { return Err(format!("{} blocks without a parent, not one", roots.len())) };
    let coords =
        |block: &Block| ["coords_l", "coords_r", "coords_t", "coords_b"].map(|name| field(&lines, block, name));
    Ok(blocks
        .iter()
        .filter(|block| {
            block.kind == "ui::group"
                && field(&lines, block, "my_parent") == Some(window.name)
                && coords(block) == coords(window)
        })
        .map(|block| block.name.to_owned())
        .collect())
}

/// Adds the WestPlay screen image `texture` to a window screen's script ([`WindowScreen`]), over the part of the
/// window its glass shows (`shown`), on top. The rest of the script stays as it is.
pub fn patch_window_script(text: &str, texture: &str, shown: &ShownPart, rows: u32) -> Result<String, String> {
    if text.contains("_nameless.westplay.screen") {
        return Err("has the screen already".into());
    }
    // Its coordinates may go up the screen or down it: its top is the top, either way. The script's coordinates are
    // whole units, a unit some pixels of the screen: each edge rounded outward, and a unit further, so none of what's
    // under it shows at the glass's edge (rounded to the nearest, the toy GPS's map showed a pixel's line at the
    // Setra's tablet's top and bottom: owner 2026-10-26).
    add_to_window(text, "_nameless.westplay.screen", texture, SCREEN_LAYER, |[l, r, t, b]| {
        let outward = |inside: f64, outside: f64, value: f64| {
            if outside >= inside { value.ceil() + 1.0 } else { value.floor() - 1.0 }
        };
        let (left, right) = (l + (r - l) * shown.left, l + (r - l) * shown.right);
        let (top, bottom) = (t - (t - b) * shown.top, t - (t - b) * shown.bottom);
        // The texture's rows beyond the frame's below the glass, as `patch_gps_script` puts them.
        let bottom = top - (top - bottom) * f64::from(rows) / f64::from(SCREEN_HEIGHT);
        [
            outward(right, left, left),
            outward(left, right, right),
            outward(bottom, top, top),
            outward(top, bottom, bottom),
        ]
    })
}

/// A window of WestPlay's for a window screen whose script the game doesn't have: 1000 units each way, nothing in it.
fn own_window() -> String {
    [
        "SiiNunit",
        "{",
        "ui::window : _nameless.westplay.window {",
        " window_handler: null",
        " clip_children: true",
        " keep_aspect: none",
        " user_string_data: \"\"",
        " first_direction_focus_id: 0",
        " fitting: false",
        " my_children: 0",
        " coords_l: 0",
        " coords_r: 1000",
        " coords_t: 1000",
        " coords_b: 0",
        " area_l: 0",
        " area_r: 1000",
        " area_t: 1000",
        " area_b: 0",
        " id: 0",
        " layer: 0",
        " tab: -1",
        " pointer: -1",
        " my_parent: null",
        "}",
        "",
        "}",
        "",
    ]
    .join("\n")
}

/// Adds `texture`'s image to a UI script as the last child of its window (its one block with no parent), as `element`,
/// where `place` puts it (left, right, top, bottom) from the window's own coordinates (the same), on `layer`.
fn add_to_window(
    text: &str,
    element: &str,
    texture: &str,
    layer: u32,
    place: impl Fn([f64; 4]) -> [f64; 4],
) -> Result<String, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(newline).collect();
    let blocks = blocks(&lines);
    let roots: Vec<&Block> = blocks.iter().filter(|block| field(&lines, block, "my_parent") == Some("null")).collect();
    let [window] = roots[..] else { return Err(format!("{} blocks without a parent, not one", roots.len())) };
    add_to(text, window.name, element, texture, layer, place)
}

/// Adds `texture`'s image to a UI script as the last child of its block `parent`, as `element`, where `place` puts it
/// (left, right, top, bottom) from the block's own coordinates (the same), on `layer`.
fn add_to(
    text: &str,
    parent: &str,
    element: &str,
    texture: &str,
    layer: u32,
    place: impl Fn([f64; 4]) -> [f64; 4],
) -> Result<String, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(newline).collect();
    let blocks = blocks(&lines);
    let window = blocks.iter().find(|block| block.name == parent).ok_or_else(|| format!("no block {parent}"))?;
    let count: usize =
        field(&lines, window, "my_children").unwrap_or("0").parse().map_err(|_| "my_children isn't a count")?;
    let coord = |name| -> Result<f64, String> {
        let text = field(&lines, window, name).ok_or_else(|| format!("the window has no {name}"))?;
        text.parse().map_err(|_| format!("the window's {name} {text}"))
    };
    let [left, right, top, bottom] =
        place([coord("coords_l")?, coord("coords_r")?, coord("coords_t")?, coord("coords_b")?]);
    let at = |value: f64| format!("{}", value.round() as i64);

    let mut body = Vec::new();
    for line in &lines[window.start + 1..window.end] {
        let trimmed = line.trim();
        let line = if trimmed.starts_with("my_children:") {
            format!(" my_children: {}", count + 1)
        } else {
            (*line).to_owned()
        };
        let after = if count == 0 {
            trimmed.starts_with("my_children:")
        } else {
            trimmed.starts_with(&format!("my_children[{}]:", count - 1))
        };
        body.push(line);
        if after {
            body.push(format!(" my_children[{count}]: {element}"));
        }
    }
    let image = [
        String::new(),
        format!("ui::text : {element} {{"),
        format!(" text: \"<img src=/{SCREEN_DIR}/{texture}.mat xscale=stretch yscale=stretch>\""),
        format!(" coords_l: {}", at(left)),
        format!(" coords_r: {}", at(right)),
        format!(" coords_t: {}", at(top)),
        format!(" coords_b: {}", at(bottom)),
        " area_l: 1".into(),
        " area_r: 0".into(),
        " area_t: 0".into(),
        " area_b: 1".into(),
        " id: 0".into(),
        format!(" layer: {layer}"),
        " tab: -1".into(),
        " pointer: -1".into(),
        format!(" my_parent: {}", window.name),
        "}".into(),
    ];
    let last_end = blocks.last().map(|block| block.end).ok_or("no blocks")?;
    let mut patched: Vec<String> = lines[..=window.start].iter().map(|line| (*line).to_owned()).collect();
    patched.extend(body);
    patched.extend(lines[window.end..=last_end].iter().map(|line| (*line).to_owned()));
    patched.extend(image);
    patched.extend(lines[last_end + 1..].iter().map(|line| (*line).to_owned()));
    Ok(patched.join(newline))
}

/// A zip of `files` (path, contents), deflated, as the game reads `.scs` mods.
pub(crate) fn zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let (mut data, mut directory) = (Vec::new(), Vec::new());
    for (path, contents) in files {
        let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(contents).expect("deflating to memory");
        let packed = encoder.finish().expect("deflating to memory");
        let mut crc = flate2::Crc::new();
        crc.update(contents);
        let local = data.len() as u32;
        // Version 2.0, no flags, deflated, no time.
        let common = |out: &mut Vec<u8>| {
            out.extend(20u16.to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(8u16.to_le_bytes());
            out.extend([0u8; 4]);
            out.extend(crc.sum().to_le_bytes());
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend((contents.len() as u32).to_le_bytes());
            out.extend((path.len() as u16).to_le_bytes());
            out.extend(0u16.to_le_bytes());
        };
        data.extend(b"PK\x03\x04");
        common(&mut data);
        data.extend(path.as_bytes());
        data.extend(&packed);
        directory.extend(b"PK\x01\x02");
        directory.extend(20u16.to_le_bytes());
        common(&mut directory);
        // No comment, disk 0, no attributes.
        directory.extend([0u8; 10]);
        directory.extend(local.to_le_bytes());
        directory.extend(path.as_bytes());
    }
    let offset = data.len() as u32;
    let size = directory.len() as u32;
    data.extend(directory);
    data.extend(b"PK\x05\x06");
    data.extend([0u8; 4]);
    data.extend((files.len() as u16).to_le_bytes());
    data.extend((files.len() as u16).to_le_bytes());
    data.extend(size.to_le_bytes());
    data.extend(offset.to_le_bytes());
    data.extend(0u16.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {

    /// The add-ons the mod carries: YTHQ's tablets in both games' trucks, the Fords' centre screens and the Ford
    /// Mustang '67 and Victoria '06 cars' YTHQ tablets in American Truck Simulator, and none of their own manifests
    /// (the mod's is WestPlay's).
    #[test]
    fn the_add_ons_are_there_for_each_games_vehicles() {
        let ets2 = addons(crate::game::Kind::Ets2);
        assert!(ets2.iter().any(|(path, _)| path == "ui/dashboard/ythq_tablet_gps.sii"), "{} files", ets2.len());
        assert!(ets2.iter().all(|(path, _)| !path.ends_with("manifest.sii")));
        let ats = addons(crate::game::Kind::Ats);
        assert!(ats.iter().any(|(path, _)| path == "ui/dashboard/ford_f150_gps_23.sii"), "{} files", ats.len());
        // Its YTHQ tablet trucks': their screens' materials (its own paths WestPlay's tablet texture, which the cars'
        // tablets sample), not their models (YTHQ's own show its UI, `the_ats_ythq_tablet_models_are_ythqs_own`).
        assert!(
            ats.iter().any(|(path, _)| path.starts_with("automat/") && path.ends_with(".mat")),
            "{} files",
            ats.len()
        );
        assert!(ats.iter().all(|(path, _)| !path.starts_with("vehicle/truck/upgrade/interior_decors/")));
        assert!(ats.iter().all(|(path, _)| path != "description.txt" && !path.ends_with("manifest.sii")));
        // The Ford Mustang '67 and Victoria '06 cars' YTHQ tablets: their models' screen faces are stretched over
        // WestPlay's whole tablet texture (the ats-ythq models' transform), so its frame isn't cut and upside down
        // (owner 2026-11-17). Only those two car models: nothing else of a car mod's.
        let cars: Vec<&String> =
            ats.iter().map(|(path, _)| path).filter(|path| path.starts_with("vehicle/car/")).collect();
        assert_eq!(cars.len(), 2, "{cars:?}");
        for name in ["mustang_67", "victoria_06"] {
            let path = format!("vehicle/car/upgrade/interior_decors/set_dashbrd/ythq_tablet_small_{name}.pmg");
            let entry = ats.iter().find(|(found, _)| *found == path).unwrap_or_else(|| panic!("no {path}"));
            let model = westplay_mapdata::pmg::Pmg::read(&entry.1)
                .unwrap_or_else(|| panic!("{path}: not a model"));
            // The screen face is the third piece (its material is WestPlay's tablet texture, the patched model's
            // own material list): 24 vertices, their UVs the whole texture now.
            let piece = model.piece(2).unwrap_or_else(|| panic!("{path}: no screen piece"));
            let mut min = [f32::INFINITY; 2];
            let mut max = [f32::NEG_INFINITY; 2];
            for vertex in 0..piece.vertices {
                let uv = model.coordinate(&piece, vertex).unwrap_or_else(|| panic!("{path}: a vertex without UVs"));
                min[0] = min[0].min(uv[0]);
                min[1] = min[1].min(uv[1]);
                max[0] = max[0].max(uv[0]);
                max[1] = max[1].max(uv[1]);
            }
            assert!(
                min[0].abs() < 0.01
                    && min[1].abs() < 0.01
                    && (max[0] - 1.0).abs() < 0.01
                    && (max[1] - 1.0).abs() < 0.01,
                "{path}: the screen face's UVs are {min:?}..{max:?}, not the whole texture"
            );
        }
        // The two games' add-ons don't cross: ETS2's get no ATS path (and the cars none), ATS's no bridge script.
        assert!(ets2.iter().all(|(path, _)| !path.starts_with("vehicle/car/") && !path.starts_with("ui/dashboard/ford")));
        assert!(ats.iter().all(|(path, _)| path != "ui/dashboard/ythq_tablet_gps.sii"));
        assert!(ats.iter().all(|(path, _)| !path.starts_with("vehicle/bus/")));
        // The Fords' backdrop left clear: their own screen shows while WestPlay doesn't draw on it.
        let ford = ats.iter().find(|(path, _)| path == "ui/dashboard/ford_f150_gps_23.sii").unwrap();
        let script = String::from_utf8_lossy(&ford.1).into_owned();
        assert!(script.contains("<color value=00000000>"), "the backdrop made clear");
        assert!(!script.contains("<color value=FF000000>"), "no opaque backdrop left");
        // The reversing camera over WestPlay's picture, where the game shows it (layer 9 against WestPlay's 101).
        assert!(script.contains("layer: 109"), "the parking camera lifted over the picture");
        // WestPlay's picture on a YTHQ tablet drawn to fit the model that shows it (`pmg_uv`: its screen face is the
        // whole texture).
        let ythq = ets2.iter().find(|(path, _)| path == "ui/dashboard/ythq_tablet_gps.sii").unwrap();
        let overlay = String::from_utf8_lossy(&ythq.1).into_owned();
        assert!(!overlay.contains("coords_l: -14"), "the picture inset on the left");
        assert!(!overlay.contains("coords_r: 526"), "the picture inset on the right");
        // What the mod carries said by a digest, so a mod built before them is made again.
        assert_ne!(addons_digest(), 0);
    }

    /// American Truck Simulator's YTHQ tablet models: none of the add-ons' (their screen faces laid out for WestPlay's
    /// tablet texture, the whole of it), YTHQ's own instead, which the mod's tablets draw its one script into the trucks'
    /// own screens' texture with: its own screen faces show that texture's middle band. The cars' YTHQ tablets stay:
    /// their screens sample WestPlay's tablet texture directly.
    #[test]
    fn the_ats_ythq_tablet_models_are_ythqs_own_not_the_add_ons() {
        let ats = addons(Kind::Ats);
        assert!(
            ats.iter().all(|(path, _)| !path.starts_with("vehicle/truck/upgrade/interior_decors/")),
            "the add-ons' YTHQ tablet models carried: {:?}",
            ats.iter().map(|(path, _)| path).collect::<Vec<_>>()
        );
        // Its screens' materials are the mod's still (YTHQ's own paths WestPlay's tablet texture, which the cars'
        // screens sample): without them the tablets' screens keep the factory screens' picture.
        let screen = ats.iter().find(|(path, _)| path == ATS_YTHQ_SCREEN_MAT).expect("its screen material");
        let text = String::from_utf8_lossy(&screen.1).into_owned();
        assert!(text.contains("screen_tablet.tobj"), "{text}");
        // The cars' YTHQ tablets: their models, and nothing else of a car mod's.
        assert_eq!(ats.iter().filter(|(path, _)| path.starts_with("vehicle/car/")).count(), 2);
    }

    use super::*;

    #[test]
    fn installs_the_mod_whole_and_says_which_step_failed_where() {
        let root = std::env::temp_dir().join(format!("westplay-install-{}", std::process::id()));
        let mod_dir = root.join("Euro Truck Simulator 2").join("mod");
        let path = install(b"mod", &mod_dir, Kind::Ets2).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"mod");
        assert_eq!(path.file_name().unwrap(), "westplay.scs");
        assert!(!mod_dir.join("westplay.scs.partial").exists());
        // ATS's beside it, ETS2's left as it is.
        let ats = install(b"ats mod", &mod_dir, Kind::Ats).unwrap();
        assert_eq!(ats.file_name().unwrap(), "westplay_ats.scs");
        assert_eq!(std::fs::read(&path).unwrap(), b"mod");
        // A file where the folder should be: the step and its path in the error.
        let blocked = root.join("blocked");
        std::fs::write(&blocked, b"").unwrap();
        let error = install(b"mod", &blocked.join("mod"), Kind::Ets2).unwrap_err().to_string();
        assert!(error.starts_with("making the folder ") && error.contains("blocked"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Optional in convoys: a friend without WestPlay can join, and be joined.
    #[test]
    fn the_mod_is_optional_in_convoys() {
        let units = westplay_mapdata::sii::parse(&manifest());
        let package = units.iter().find(|unit| unit.class == "mod_package").expect("a mod_package");
        assert_eq!(package.get("mp_mod_optional"), Some("true"));
    }

    /// A GPS script laid out as the game's: a window, the "screen on" group with two children, and them.
    const SCRIPT: &str = "SiiNunit\n{\nui::window : _nameless.1 {\n my_children: 1\n my_children[0]: _nameless.2\n}\n\nui::group : _nameless.2 {\n coords_l: 0\n coords_r: 1279\n coords_t: 719\n coords_b: 0\n id: 100\n my_children: 2\n my_children[0]: _nameless.3\n my_children[1]: _nameless.4\n my_parent: _nameless.1\n}\n\nui::text : _nameless.3 {\n my_parent: _nameless.2\n}\n\nui::text : _nameless.4 {\n my_parent: _nameless.2\n}\n\n}\n";

    #[test]
    fn adds_the_screen_on_top_of_the_screen_on_group() {
        let patched = patch_gps_script(SCRIPT, "screen_generic", None, SCREEN_HEIGHT).unwrap();
        assert!(patched.contains(" my_children: 3\n my_children[0]: _nameless.3\n my_children[1]: _nameless.4\n my_children[2]: _nameless.westplay.screen\n"));
        assert!(patched.contains("ui::text : _nameless.westplay.screen {\n text: \"<img src=/material/ui/westplay/screen_generic.mat xscale=stretch yscale=stretch>\"\n coords_l: 0\n coords_r: 1279\n coords_t: 719\n coords_b: 0\n"));
        assert!(patched.contains(" layer: 100\n tab: -1\n pointer: -1\n my_parent: _nameless.2\n}\n\n}\n"));
        // The window still has its one child; the rest is as it was.
        assert!(patched.starts_with("SiiNunit\n{\nui::window : _nameless.1 {\n my_children: 1\n"));
    }

    #[test]
    fn reads_a_scripts_blocks_however_theyre_indented() {
        // A mod's scripts indent their blocks (YTHQ Tablet's every one): still the game's layout to patch.
        let indented = SCRIPT.replace("\nui::", "\n\tui::").replace("\n my_", "\n\t\tmy_");
        let patched = patch_gps_script(&indented, "screen_generic", None, SCREEN_HEIGHT).unwrap();
        assert!(patched.contains(" my_children[2]: _nameless.westplay.screen\n"), "{patched}");
        assert!(patched.contains(" my_parent: _nameless.2\n"), "{patched}");
    }

    /// A coach's GPS script as the Coaches DLC's (the MAN Lion's Coach's, shortened): the "screen on" group and the
    /// parking camera's beside it in the window, the camera a background, its picture and, in a group of their own,
    /// its marks.
    const CAMERA_SCRIPT: &str = "SiiNunit\n{\nui::window : _nameless.1 {\n my_children: 2\n my_children[0]: _nameless.on\n my_children[1]: _nameless.cam\n layer: 0\n}\n\nui::group : _nameless.on {\n coords_l: 0\n coords_r: 800\n coords_t: 480\n coords_b: 0\n id: 100\n my_children: 1\n my_children[0]: _nameless.map\n layer: 1\n my_parent: _nameless.1\n}\n\nui::text : _nameless.map {\n layer: 2\n my_parent: _nameless.on\n}\n\nui::group : _nameless.cam {\n coords_l: 0\n coords_r: 800\n coords_t: 480\n coords_b: 0\n id: 910\n my_children: 3\n my_children[0]: _nameless.bg\n my_children[1]: _nameless.pic\n my_children[2]: _nameless.marks\n layer: 1\n my_parent: _nameless.1\n}\n\nui::text : _nameless.bg {\n layer: 3\n my_parent: _nameless.cam\n}\n\nui::text : _nameless.pic {\n layer: 4\n my_parent: _nameless.cam\n}\n\nui::group : _nameless.marks {\n my_children: 1\n my_children[0]: _nameless.arrows\n layer: 0\n my_parent: _nameless.cam\n}\n\nui::text : _nameless.arrows {\n layer: 5\n my_parent: _nameless.marks\n}\n\n}\n";

    #[test]
    fn the_parking_camera_is_drawn_over_the_screen_and_the_screen_over_the_rest() {
        let patched = patch_gps_script(CAMERA_SCRIPT, "screen_man_lions_24", None, SCREEN_HEIGHT).unwrap();
        let layer = |name: &str| {
            let block = patched.split(&format!(" : {name} {{")).nth(1).unwrap_or_else(|| panic!("{name}"));
            let line = block.lines().find(|line| line.trim().starts_with("layer:")).unwrap();
            line.trim()["layer:".len()..].trim().parse::<u32>().unwrap()
        };
        let screen = layer("_nameless.westplay.screen");
        assert_eq!(screen, SCREEN_LAYER);
        // The game's map, and the groups it's in, under the screen as they were.
        assert_eq!((layer("_nameless.on"), layer("_nameless.map"), layer("_nameless.1")), (1, 2, 0));
        // The camera, at every depth, over it; in its own order.
        let camera =
            ["_nameless.cam", "_nameless.bg", "_nameless.pic", "_nameless.marks", "_nameless.arrows"].map(layer);
        assert!(camera.iter().all(|&layer| layer > screen), "{camera:?}");
        assert!(layer("_nameless.bg") < layer("_nameless.pic") && layer("_nameless.pic") < layer("_nameless.arrows"));
        // The screen still in the "screen on" group.
        assert!(patched.contains(" my_children[1]: _nameless.westplay.screen\n"));
        // The camera's mark in the camera's group (its last child), drawn only with it: transparent, in its corner.
        assert!(patched.contains(" my_children: 4\n my_children[0]: _nameless.bg\n"), "{patched}");
        assert!(patched.contains(" my_children[3]: _nameless.westplay.parking\n"));
        let mark = patched.split("ui::text : _nameless.westplay.parking {").nth(1).unwrap();
        assert!(mark.contains("<img src=/material/ui/westplay/parking_mark.mat"));
        assert!(mark.contains(" my_parent: _nameless.cam\n"));
        // No mark where there's no camera.
        let plain = patch_gps_script(SCRIPT, "screen_generic", None, SCREEN_HEIGHT).unwrap();
        assert!(!plain.contains("westplay.parking"));
        // Lifting changes the camera's five layers and nothing else.
        let lifted = lift_parking_camera(CAMERA_SCRIPT).unwrap();
        let changed: Vec<(&str, &str)> =
            CAMERA_SCRIPT.lines().zip(lifted.lines()).filter(|(before, after)| before != after).collect();
        assert_eq!(CAMERA_SCRIPT.lines().count(), lifted.lines().count());
        assert_eq!(changed.len(), 5, "{changed:?}");
        assert!(changed.iter().all(|(before, after)| before.starts_with(" layer:") && after.starts_with(" layer:")));
    }

    #[test]
    fn a_script_without_a_parking_camera_keeps_its_layers() {
        assert_eq!(lift_parking_camera(SCRIPT).unwrap(), SCRIPT);
        let crlf = CAMERA_SCRIPT.replace('\n', "\r\n");
        let lifted = lift_parking_camera(&crlf).unwrap();
        assert!(lifted.contains("ui::text : _nameless.pic {\r\n layer: 105\r\n"), "{lifted}");
        // A camera element without a layer, or with one that isn't a number: said, not guessed.
        assert!(lift_parking_camera(&CAMERA_SCRIPT.replace(" layer: 4\n", "")).is_err());
        assert!(lift_parking_camera(&CAMERA_SCRIPT.replace(" layer: 4\n", " layer: top\n")).is_err());
    }

    #[test]
    fn puts_the_screen_on_the_part_the_display_shows() {
        // The 7" Scania's: the stock square's full width, rows 12.2%-73.5% from its top.
        let square = SCRIPT.replace("coords_r: 1279", "coords_r: 256").replace("coords_t: 719", "coords_t: 256");
        let shown = SHOWN_PARTS.iter().find(|(id, _)| *id == "generic").map(|(_, part)| part);
        let patched = patch_gps_script(&square, "screen_generic", shown, SCREEN_HEIGHT).unwrap();
        let placed = "screen_generic.mat xscale=stretch yscale=stretch>\"\n coords_l: 1\n coords_r: 255\n coords_t: 225\n coords_b: 68\n";
        assert!(patched.contains(placed), "{patched}");
        // The group itself keeps its coordinates.
        assert!(patched.contains(" coords_r: 256\n coords_t: 256\n coords_b: 0\n id: 100\n"));
    }

    #[test]
    fn puts_the_frames_rows_on_the_screen_and_the_textures_extra_rows_below_it() {
        // The Volvo FH's: its texture 726 rows (its index 6), its display showing all but 0.54-0.55% of each edge.
        let script = SCRIPT.replace("coords_r: 1279", "coords_r: 1280").replace("coords_t: 719", "coords_t: 720");
        let shown = SHOWN_PARTS.iter().find(|(id, _)| *id == "volvo-fh").map(|(_, part)| part);
        let patched = patch_gps_script(&script, "screen_volvo_fh", shown, 726).unwrap();
        // The part shown: 7 to 1273 across, 716 to 4 up; the frame's 720 rows over that, so the texture's 726 reach
        // 6 rows' worth (5.9 units) further down, off the screen.
        let placed = "coords_l: 7\n coords_r: 1273\n coords_t: 716\n coords_b: -2\n";
        assert!(patched.contains(placed), "{patched}");
        // A texture the frame's size: on the part exactly.
        let patched = patch_gps_script(&script, "screen_volvo_fh", shown, SCREEN_HEIGHT).unwrap();
        assert!(patched.contains("coords_l: 7\n coords_r: 1273\n coords_t: 716\n coords_b: 4\n"), "{patched}");
    }

    #[test]
    fn the_s_bevs_screens_file_gets_the_sharp_size() {
        let root = std::env::temp_dir().join(format!("westplay-sharp-bev-{}", std::process::id()));
        let dir = root.join("def/vehicle/truck/scania.s_2024e/interior");
        std::fs::create_dir_all(&dir).unwrap();
        let files = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(root.clone())]);
        let path = SHARP_SCREENS[0];
        // Not in the game (its DLC not owned): nothing to make.
        assert_eq!(sharp_screens_file(&files, path), Ok(None));
        // As the game has it: the size line made the sharp one, the rest as it was.
        let stock = "\tdashboard_path: \"/ui/dashboard/scania_s_2024e.sii\"\r\n\r\n\tgps_path: \"/ui/gps.sii\"\r\n\tgps_drawable_size: (256, 256)\r\n";
        std::fs::write(root.join(path), stock).unwrap();
        let sharp = String::from_utf8(sharp_screens_file(&files, path).unwrap().unwrap()).unwrap();
        assert_eq!(sharp, stock.replace("(256, 256)", "(1024, 1024)"));
        // Another screen named: left as the game has it.
        std::fs::write(root.join(path), stock.replace("/ui/gps.sii", "/ui/dashboard/other_gps.sii")).unwrap();
        assert!(sharp_screens_file(&files, path).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_coaches_screens_files_draw_their_screens_at_the_volvo_fhs_size() {
        let root = std::env::temp_dir().join(format!("westplay-sharp-coaches-{}", std::process::id()));
        let files = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(root.clone())]);
        // As the Coaches DLC has them (2026-11-12): the Scania's GPS first, the MAN's after its dashboard.
        let scania = "	gps_path: \"/ui/dashboard/scania_touring_23_gps.sii\"
	gps_drawable_size: (512, 256)
                      	dashboard_path: \"/ui/dashboard/scania_touring_23.sii\"
	dashboard_drawable_size: (512, 512)
";
        let man = "	dashboard_path: \"/ui/dashboard/man_lions_24.sii\"
	dashboard_drawable_size: (1024, 512)
                   	gps_path: \"/ui/dashboard/man_lions_24_gps.sii\"
	gps_drawable_size: (512, 256)
";
        for (&(path, script), text) in BUS_SCREENS.iter().zip([scania, man]) {
            // Not in the game (the DLC not owned): nothing to make.
            assert_eq!(sized_screens_file(&files, path, script, BUS_SIZE), Ok(None));
            std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
            std::fs::write(root.join(path), text).unwrap();
            // The GPS screen's size made the Volvo FH's, the dashboard's as it was.
            let sized = sized_screens_file(&files, path, script, BUS_SIZE).unwrap().unwrap();
            let sized = String::from_utf8(sized).unwrap();
            let gps = format!("gps_path: \"/{script}\"");
            assert_eq!(sized, text.replace("gps_drawable_size: (512, 256)", "gps_drawable_size: (1024, 512)"));
            assert!(sized.contains(&gps));
            // Another screen named: left as the game has it.
            std::fs::write(root.join(path), text.replace(script, "ui/dashboard/other_gps.sii")).unwrap();
            assert!(sized_screens_file(&files, path, script, BUS_SIZE).is_err());
        }
        // Each a layout's: the coaches' screens WestPlay's.
        for (_, script) in BUS_SCREENS {
            assert!(screen_layouts().iter().any(|layout| layout.gps_scripts.contains(script)), "{script}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_7_inch_interiors_get_a_sharp_screen() {
        let root = smart_dash_game("sharp", "/ui/dashboard/scania_2025_gps.sii");
        for truck in ["s", "r"] {
            let unit = format!("{truck}_high_black_dark");
            let path = root.join(format!("def/vehicle/truck/scania.{truck}_2016/interior/cab_{unit}.sii"));
            let text = format!(
                "SiiNunit\r\n{{\r\naccessory_interior_data: {unit}\r\n{{\r\n\tdashboard_path: \"/ui/dashboard/scania_2016_std.sii\"\r\n\r\n\tglass_panes[]: x\r\n}}\r\n}}\r\n"
            );
            std::fs::write(path, text).unwrap();
            // An exclusive interior names the stock screen itself.
            let exclusive =
                root.join(format!("def/vehicle/truck/scania.{truck}_2016/interior/cab_{truck}_high_v8.sii"));
            let text = "SiiNunit\r\n{\r\naccessory_interior_data: v8\r\n{\r\n\tdashboard_path: \"/ui/dashboard/scania_2016_exc.sii\"\r\n\tgps_path: \"/ui/gps.sii\"\r\n}\r\n}\r\n";
            std::fs::write(exclusive, text).unwrap();
        }
        let files = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(root.clone())]);
        let (bytes, built) = build(&files, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
        assert_eq!(built.not_sharp, None);
        let out = root.join("westplay.scs");
        std::fs::write(&out, bytes).unwrap();
        let archive = westplay_mapdata::archive::Archive::open(&out).unwrap();
        let text = |path: &str| archive.read(path).unwrap().map(|bytes| String::from_utf8(bytes).unwrap());
        for truck in ["s", "r"] {
            let dir = format!("def/vehicle/truck/scania.{truck}_2016/interior");
            let interior = text(&format!("{dir}/cab_{truck}_high_black_dark.sii")).unwrap();
            let sized = "\tdashboard_path: \"/ui/dashboard/scania_2016_std.sii\"\r\n\tgps_drawable_size: (1024, 1024)\r\n\r\n\tglass_panes[]: x\r\n";
            assert!(interior.contains(sized), "{interior}");
            let v8 = text(&format!("{dir}/cab_{truck}_high_v8.sii")).unwrap();
            let sized = "exc.sii\"\r\n\tgps_drawable_size: (1024, 1024)\r\n\tgps_path: \"/ui/gps.sii\"\r\n";
            assert!(v8.contains(sized), "{v8}");
            // The Smart Dash's own screens are left as they are.
            assert_eq!(text(&format!("{dir}/cab_{truck}_high_smartdash_dark.sii")), None);
        }
        drop(archive);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn marks_a_dashboard_in_its_windows_corner_once() {
        let script = "SiiNunit\r\n{\r\nui::window : _nameless.w {\r\n my_children: 0\r\n coords_l: 10\r\n coords_r: 500\r\n coords_t: 0\r\n coords_b: 300\r\n my_parent: null\r\n}\r\n\r\n}\r\n";
        let patched = patch_dashboard_script(script, "cab_mark").unwrap();
        assert!(patched.contains(" my_children: 1\r\n my_children[0]: _nameless.westplay.cab\r\n"), "{patched}");
        // Its coordinates going down the screen: up from the bottom corner, 1/64 of the window each way (rounded up).
        assert!(
            patched.contains(" coords_l: 10\r\n coords_r: 18\r\n coords_t: 295\r\n coords_b: 300\r\n"),
            "{patched}"
        );
        assert!(
            patched.ends_with("my_parent: _nameless.w\r\n}\r\n\r\n}\r\n") && !patched.contains("\r\r"),
            "{patched}"
        );
        assert!(patch_dashboard_script(&patched, "cab_mark").is_err());
        // Two windows: which is the dashboard's isn't known.
        let two = script.replace("\r\n\r\n}", "\r\nui::window : _nameless.v {\r\n my_parent: null\r\n}\r\n}");
        assert!(patch_dashboard_script(&two, "cab_mark").is_err());
    }

    /// A newer dashboard's displays, groups over the whole window: the mark in each of them too, and some pixels big.
    #[test]
    fn marks_each_display_of_a_dashboard_that_has_them() {
        let script = "SiiNunit\n{\nui::window : _nameless.wnd {\n my_children: 2\n my_children[0]: _nameless.d1\n my_children[1]: _nameless.small\n coords_l: 0\n coords_r: 1440\n coords_t: 540\n coords_b: 0\n my_parent: null\n}\n\nui::group : _nameless.d1 {\n my_children: 0\n coords_l: 0\n coords_r: 1440\n coords_t: 540\n coords_b: 0\n my_parent: _nameless.wnd\n}\n\nui::group : _nameless.small {\n my_children: 0\n coords_l: 0\n coords_r: 100\n coords_t: 540\n coords_b: 0\n my_parent: _nameless.wnd\n}\n\n}\n";
        let patched = patch_dashboard_script(script, "cab_mark").unwrap();
        assert!(patched.contains("ui::group : _nameless.d1 {\n my_children: 1\n my_children[0]: _nameless.westplay.cab0\n"), "{patched}");
        assert!(patched.contains("my_parent: _nameless.d1\n"), "{patched}");
        // Not in a group that covers only part of the window.
        assert!(!patched.contains("my_parent: _nameless.small\n}\n\nui::text"), "{patched}");
        assert_eq!(patched.matches("<img src=/material/ui/westplay/cab_mark.mat").count(), 2, "{patched}");
        // 1/64 of 1440 by 540, rounded up: 23 by 9 units, from the bottom left corner up.
        assert!(patched.contains(" coords_l: 0\n coords_r: 23\n coords_t: 9\n coords_b: 0\n"), "{patched}");
    }

    #[test]
    fn keeps_a_scripts_windows_line_endings() {
        let patched = patch_gps_script(&SCRIPT.replace('\n', "\r\n"), "screen_generic", None, SCREEN_HEIGHT).unwrap();
        assert!(patched.contains(" my_children[2]: _nameless.westplay.screen\r\n") && !patched.contains("\r\r"));
    }

    #[test]
    fn refuses_a_script_without_a_screen_on_group() {
        assert_eq!(
            patch_gps_script(&SCRIPT.replace("id: 100", "id: 7"), "x", None, SCREEN_HEIGHT),
            Err("no group with id 100".into())
        );
    }

    #[test]
    fn makes_a_transparent_texture_of_each_layouts_size() {
        let dds = transparent_dds(1280, 721);
        assert_eq!(&dds[..4], b"DDS ");
        assert_eq!(u32::from_le_bytes(dds[12..16].try_into().unwrap()), 721);
        assert_eq!(u32::from_le_bytes(dds[128..132].try_into().unwrap()), 91);
        assert_eq!(dds.len(), 148 + 1280 * 721 * 4);
        assert!(dds[148..].iter().all(|&byte| byte == 0));
    }

    /// The real thing: the mod built from the game installed on this PC (written to the temp folder, not the
    /// game's). `cargo test -p westplay-setup -- --ignored --nocapture builds_the_mod`
    #[test]
    #[ignore = "reads the PC's game"]
    fn builds_the_mod_from_the_installed_game() {
        let game = crate::game::find(crate::game::Kind::Ets2).into_iter().next().expect("the game installed");
        let files = GameFiles::install(&game.dir).unwrap();
        let (bytes, built) = build(&files, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
        let out = std::env::temp_dir().join(file_name(game.kind));
        std::fs::write(&out, &bytes).unwrap();
        println!("{} bytes, patched {:?}, missing {:?}: {}", bytes.len(), built.patched, built.missing, out.display());
        let archive = westplay_mapdata::archive::Archive::open(&out).unwrap();
        for layout in screen_layouts().iter().filter(|layout| built.patched.contains(&layout.id)) {
            for script in layout.gps_scripts.iter().filter_map(|script| archive.read(script).unwrap()) {
                let script = String::from_utf8(script).unwrap();
                assert!(script.contains("_nameless.westplay.screen"), "{}", layout.id);
            }
        }
        // The tablet, in the game's trucks.
        assert!(built.patched.contains(&TABLET));
        let actros = "def/vehicle/truck/mercedes.actros2014/accessory/toyhang/westplay_tablet.sii";
        assert!(archive.read(actros).unwrap().is_some());
        assert!(archive.read("vehicle/truck/upgrade/westplay_tablet/westplay_tablet_hang.pmg").unwrap().is_some());
    }

    /// The real thing: YTHQ Tablet's own screens and tablets read from this PC's Workshop archive, patched.
    /// `cargo test -p westplay-setup -- --ignored --nocapture reads_ythq_tablets_screens`
    #[test]
    #[ignore = "reads the PC's game"]
    fn reads_ythq_tablets_own_screens_from_this_pcs_workshop() {
        let game = crate::game::find(crate::game::Kind::Ets2).into_iter().next().expect("the game installed");
        let workshop = game.workshop().expect("the game's Workshop folder");
        let made = ythq_screen_files(Some(&workshop), game.kind);
        let by_path: std::collections::BTreeMap<&str, &Vec<u8>> =
            made.files.iter().map(|(path, bytes)| (path.as_str(), bytes)).collect();
        println!(
            "{} screens: {:?}\n{} tablets: {:?}\n{} definitions: {:?}\n{} materials: {:?}\nproblems: {:?}",
            made.screens.len(),
            made.screens,
            made.tablets.len(),
            made.tablets,
            made.accessories.len(),
            made.accessories,
            made.materials.len(),
            made.materials,
            made.problems
        );
        assert!(made.problems.is_empty(), "{:?}", made.problems);
        assert_eq!(
            made.screens,
            ythq_screens(game.kind).iter().map(|(path, _)| *path).collect::<Vec<_>>()
        );
        // Its tablets' models' screen materials: the trucks' own screens' texture object no more, its tablets' own
        // instead (what every tablet's definition draws its UI into).
        assert!(!made.materials.is_empty());
        for path in &made.materials {
            let text = String::from_utf8(by_path[path.as_str()].clone()).unwrap();
            assert!(text.contains(&format!("\"/{YTHQ_TABLET_UI}\"")), "{path}: {text}");
            assert!(!text.contains(&format!("\"/{YTHQ_FACTORY_UI}\"")), "{path}: {text}");
        }
        // A copy for its own screens' scripts and for the ones its tablets name besides them: every script of its that
        // a tablet of its draws.
        let scripts: Vec<&str> =
            ythq_screens(game.kind).iter().map(|(script, _)| *script).chain(YTHQ_TABLETS.iter().copied()).collect();
        for script in scripts {
            assert!(made.tablets.contains(&ythq_tablet_path(script)), "{script}: no copy");
        }
        // Its factory screens' scripts: WestPlay's truck picture alone, and none of the tablet's.
        for script in &made.screens {
            let text = String::from_utf8(by_path[*script].clone()).unwrap();
            assert!(text.contains("_nameless.westplay.screen"), "{script}");
            assert!(!text.contains("screen_tablet.mat"), "{script}");
            assert!(!text.contains(TABLET_OVERLAY), "{script}");
        }
        // Its tablets' copies: the tablet's picture on the window whole, and none of the truck screen's.
        for path in &made.tablets {
            let text = String::from_utf8(by_path[path.as_str()].clone()).unwrap();
            assert!(text.contains("screen_tablet.mat"), "{path}");
            assert!(!text.contains("_nameless.westplay.screen"), "{path}");
            let marker = format!("ui::text : {TABLET_OVERLAY} {{");
            let tablet = text.split(&marker).nth(1).unwrap_or_else(|| panic!("{path}"));
            assert!(tablet.contains(&format!(" layer: {TABLET_LAYER}\n")), "{path}");
            // A child of the window, as the screen's is of the "screen on" group.
            let window = text.split("ui::window : ").nth(1).and_then(|rest| rest.split(" {").next()).unwrap();
            assert!(tablet.contains(&format!(" my_parent: {window}\n")), "{path}: not in {window}");
            let coords: Vec<&str> =
                ["coords_l", "coords_r", "coords_t", "coords_b"].map(|name| tablet_lines(&tablet, name)).to_vec();
            assert_eq!(coords, window_coords(&text), "{path}");
        }
        // Its tablets' definitions: every one of them pointed at a copy of its, with the size and texture object YTHQ
        // gives it.
        assert!(!made.accessories.is_empty());
        for path in &made.accessories {
            let text = String::from_utf8(by_path[path.as_str()].clone()).unwrap();
            let ui_paths = ythq_ui_paths(&text).unwrap();
            let ours: Vec<&String> =
                ui_paths.iter().filter(|ui_path| ui_path.starts_with(&format!("/{YTHQ_TABLET_DIR}/"))).collect();
            assert_eq!(ours.len(), ui_paths.len(), "{path}: {ui_paths:?}");
            assert!(text.contains("ui_drawable_size: (512, 512)"), "{path}");
            assert!(text.contains("accessory_addon_int_ui_data"), "{path}");
            // Its tablet's own UI texture, the one the models' screen materials sample.
            let ui = format!("/{YTHQ_TABLET_UI}");
            assert_eq!(ythq_texture_path(&text).as_deref(), Some(ui.as_str()), "{path}");
        }
        // Each vehicle's tablets, and the script its screen draws: what the report shows.
        let mut vehicles: std::collections::BTreeMap<String, (usize, std::collections::BTreeSet<String>)> =
            std::collections::BTreeMap::new();
        for path in &made.accessories {
            let parts: Vec<&str> = path.split('/').collect();
            let vehicle = format!("{}/{}", parts[2], parts[3]);
            let text = String::from_utf8(by_path[path.as_str()].clone()).unwrap();
            let ui_paths = ythq_ui_paths(&text).unwrap();
            let entry = vehicles.entry(vehicle).or_default();
            for ui_path in &ui_paths {
                entry.0 += 1;
                entry.1.insert(ui_path.trim_start_matches('/').to_owned());
            }
        }
        for (vehicle, (count, scripts)) in &vehicles {
            println!("{vehicle}: {count} tablets -> {scripts:?}");
        }
        println!("{} vehicles: {} definitions", vehicles.len(), made.accessories.len());
    }

    /// The real thing, American Truck Simulator's own YTHQ Tablet: its one screen script (patched with WestPlay's two
    /// pictures, its own UI kept) and its tablets' screen materials, read from this PC's Workshop archive of its own item
    /// (`3598886740`). `cargo test -p westplay-setup -- --ignored --nocapture reads_ats_ythq`
    #[test]
    #[ignore = "reads the PC's game"]
    fn reads_ats_ythq_tablets_screens_from_this_pcs_workshop() {
        let game = crate::game::find(crate::game::Kind::Ats).into_iter().next().expect("the game installed");
        let workshop = game.workshop().expect("the game's Workshop folder");
        let made = ythq_screen_files(Some(&workshop), game.kind);
        println!(
            "{} screens: {:?}\n{} tablets: {:?}\n{} definitions read\n{} materials: {:?}\n{} models\nproblems: {:?}",
            made.screens.len(),
            made.screens,
            made.tablets.len(),
            made.tablets,
            made.accessories.len(),
            made.materials.len(),
            made.materials,
            made.models.len(),
            made.problems
        );
        assert!(made.problems.is_empty(), "{:?}", made.problems);
        // Its one script for every truck's factory screen and its tablets alike: its own, patched, no copies; its
        // definitions and models its own too.
        assert_eq!(made.screens, ["ui/dashboard/ythq_tablet_gps_mirror_h.sii"]);
        assert!(made.tablets.is_empty(), "{:?}", made.tablets);
        assert!(made.accessories.is_empty(), "{:?}", made.accessories);
        // Its tablets' screen materials (its own automat paths), each at a path of the mod's own.
        assert_eq!(made.materials.len(), 3, "{:?}", made.materials);
        let by_path: std::collections::BTreeMap<&str, &Vec<u8>> =
            made.files.iter().map(|(path, bytes)| (path.as_str(), bytes)).collect();
        let text = |path: &str| String::from_utf8(by_path[path].clone()).unwrap();
        // Its script: WestPlay's truck screen picture over its window whole (the trucks' own screens aren't WestPlay's
        // in American Truck Simulator), WestPlay's tablet picture on the band its tablets' screens show, and YTHQ's own
        // elements all kept.
        let screen = text("ui/dashboard/ythq_tablet_gps_mirror_h.sii");
        assert!(screen.contains("screen_generic.mat xscale=stretch yscale=stretch"), "{screen}");
        assert!(screen.contains(" coords_l: 0\n coords_r: 1024\n coords_t: 1024\n coords_b: 0\n"), "{screen}");
        assert!(screen.contains(TABLET_OVERLAY), "{screen}");
        assert!(screen.contains(" coords_l: 0\n coords_r: 1024\n coords_t: 827\n coords_b: 197\n"), "{screen}");
        for element in [
            "_nameless._.gpsbsd",
            "_nameless._.r_mirror",
            "_nameless.close",
            "_nameless._.speedlimit",
            "_nameless._.compass",
            "/material/ui/dashboard/bare_map.mat",
        ] {
            assert!(screen.contains(element), "{element} not kept: {screen}");
        }
        // Its screen materials: the trucks' own screens' texture, as YTHQ has them (its one script draws there, its
        // tablets' screens sample it).
        let materials: Vec<&str> =
            made.files.iter().map(|(path, _)| path.as_str()).filter(|path| path.ends_with(".mat")).collect();
        assert_eq!(materials.len(), made.materials.len(), "{materials:?}");
        for path in &materials {
            let material = text(path);
            assert!(material.contains(&format!("\"/{YTHQ_FACTORY_UI}\"")), "{path}: {material}");
            assert!(!material.contains(&format!("\"/{YTHQ_TABLET_UI}\"")), "{path}: {material}");
        }
        // Its tablets' models of every truck (its own, whose screens' materials the mod carries at paths of its own: the
        // add-ons' models of them are left out), each naming one of those.
        assert!(made.models.len() > 60, "{} models", made.models.len());
        for model in &made.models {
            let pmd = by_path.get(model.as_str()).unwrap_or_else(|| panic!("{model} not carried"));
            let found = materials
                .iter()
                .any(|material| pmd.windows(material.len()).any(|window| window == material.as_bytes()))
                && !pmd.windows(ATS_YTHQ_SCREEN_MAT.len()).any(|window| window == ATS_YTHQ_SCREEN_MAT.as_bytes());
            assert!(found, "{model}: its screen material not the mod's own");
        }
    }

    /// The `coords_…` lines of a block's text, as `<name>: <value>`.
    fn tablet_lines<'a>(text: &'a str, name: &str) -> &'a str {
        text.lines().find_map(|line| line.trim().strip_prefix(name)).map_or("", |rest| {
            rest.trim_start().strip_prefix(':').map_or("", str::trim)
        })
    }

    /// A script's window's coordinates (`coords_l` to `coords_b`), in order.
    fn window_coords(text: &str) -> Vec<&str> {
        let window = text.split("ui::window : ").nth(1).and_then(|rest| rest.split(" {").next()).unwrap_or("");
        let start = text.find(&format!("ui::window : {window} {{")).unwrap_or(0);
        let block = &text[start..];
        ["coords_l", "coords_r", "coords_t", "coords_b"].map(|name| tablet_lines(block, name)).to_vec()
    }

    /// A game folder with the Smart Dash's script and screens file, and an Advanced and a 10.1" interior in
    /// each truck's folder (`gps_path`: what its screens file names).
    fn smart_dash_game(name: &str, gps_path: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("westplay-own-screen-{name}-{}", std::process::id()));
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("ui/dashboard/scania_2025_gps.sii", SCRIPT);
        for truck in ["s", "r"] {
            let dir = format!("def/vehicle/truck/scania.{truck}_2016/interior");
            write(
                &format!("{dir}/computers_2025.sui"),
                &format!("\tgps_path:\"{gps_path}\"\r\n\tgps_drawable_size: (1024, 512)\r\n"),
            );
            let interior = |unit: &str| {
                format!(
                    "SiiNunit\r\n{{\r\naccessory_interior_data: {unit}\r\n{{\r\n@include \"computers_2025.sui\"\r\n}}\r\n}}\r\n"
                )
            };
            write(&format!("{dir}/cab_{truck}_high_smartdash_advanced_dark.sii"), &interior("a_sda_drk"));
            write(&format!("{dir}/cab_{truck}_high_smartdash_dark.sii"), &interior("a_sd_drk"));
        }
        root
    }

    #[test]
    fn a_dlc_installed_after_the_mod_was_made_has_it_made_again() {
        let root = smart_dash_game("dlc-later", "/ui/dashboard/scania_2025_gps.sii");
        let files = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(root.clone())]);
        let out = root.join("westplay.scs");
        let make = || {
            let (bytes, built) = build(&files, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
            std::fs::write(&out, bytes).unwrap();
            built
        };
        // Made without the Coaches DLC: as the game has its screens.
        assert!(!make().patched.contains(&"man-lions-24"));
        assert!(game_screens_current(&files, &out));
        // The DLC installed since (a player's, 2026-11-15): the MAN Lion's Coach's script and screens file.
        let write = |path: &str, text: &str| {
            std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
            std::fs::write(root.join(path), text).unwrap();
        };
        write("ui/dashboard/man_lions_24_gps.sii", CAMERA_SCRIPT);
        let (screens, _) = BUS_SCREENS[1];
        write(
            screens,
            "\tdashboard_path: \"/ui/dashboard/man_lions_24.sii\"\n\tgps_path: \"/ui/dashboard/man_lions_24_gps.sii\"\n\
             \tgps_drawable_size: (512, 256)\n",
        );
        assert!(!game_screens_current(&files, &out));
        // Made again: the coach's screen WestPlay's, drawn at the coaches' size, and the mod as the game has it.
        assert!(make().patched.contains(&"man-lions-24"));
        assert!(game_screens_current(&files, &out));
        let archive = westplay_mapdata::archive::Archive::open(&out).unwrap();
        let sized = String::from_utf8(archive.read(screens).unwrap().unwrap()).unwrap();
        assert!(sized.contains(BUS_SIZE), "{sized}");
        // A game update changing a script: not as the game has it any more.
        write("ui/dashboard/man_lions_24_gps.sii", &CAMERA_SCRIPT.replace("layer: 5", "layer: 6"));
        assert!(!game_screens_current(&files, &out));
        // The DLC gone again: its screen in the mod is no longer the game's.
        std::fs::remove_file(root.join("ui/dashboard/man_lions_24_gps.sii")).unwrap();
        std::fs::remove_file(root.join(screens)).unwrap();
        assert!(!game_screens_current(&files, &out));
        make();
        assert!(game_screens_current(&files, &out));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_smart_dash_advanced_gets_a_screen_of_its_own() {
        let root = smart_dash_game("whole", "/ui/dashboard/scania_2025_gps.sii");
        let files = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(root.clone())]);
        let (bytes, built) = build(&files, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
        assert!(built.patched.contains(&"scania-2025") && built.patched.contains(&"scania-2025-advanced"));
        // No 7" interiors here (the Smart Dash's alone): the build says the 7" screen stays as the game has it.
        assert_eq!(built.not_sharp.as_deref(), Some("no 7\" interiors"));
        let out = root.join("westplay.scs");
        std::fs::write(&out, bytes).unwrap();
        let archive = westplay_mapdata::archive::Archive::open(&out).unwrap();
        let text = |path: &str| archive.read(path).unwrap().map(|bytes| String::from_utf8(bytes).unwrap());
        // Its own script, patched with its own texture.
        let script = text("ui/dashboard/westplay_scania_2025_advanced_gps.sii").unwrap();
        assert!(script.contains("/material/ui/westplay/screen_scania_2025_advanced.mat"), "{script}");
        for truck in ["s", "r"] {
            let dir = format!("def/vehicle/truck/scania.{truck}_2016/interior");
            let screens = text(&format!("{dir}/westplay_computers_2025_advanced.sui")).unwrap();
            assert_eq!(
                screens,
                "\tgps_path:\"/ui/dashboard/westplay_scania_2025_advanced_gps.sii\"\r\n\tgps_drawable_size: (1024, 512)\r\n"
            );
            let advanced = text(&format!("{dir}/cab_{truck}_high_smartdash_advanced_dark.sii")).unwrap();
            assert!(advanced.contains("@include \"westplay_computers_2025_advanced.sui\"\r\n"), "{advanced}");
            assert!(advanced.contains("accessory_interior_data: a_sda_drk\r\n"));
            // The 10.1" isn't touched.
            assert_eq!(text(&format!("{dir}/cab_{truck}_high_smartdash_dark.sii")), None);
        }
        drop(archive);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_own_screen_not_as_read_is_left_out_whole() {
        // A screens file naming another script (a game update): no Advanced screen, and nothing of it.
        let root = smart_dash_game("unlike", "/ui/dashboard/scania_2026_gps.sii");
        let files = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(root.clone())]);
        let (bytes, built) = build(&files, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
        assert!(built.patched.contains(&"scania-2025") && built.missing.contains(&"scania-2025-advanced"));
        let out = root.join("westplay.scs");
        std::fs::write(&out, bytes).unwrap();
        let archive = westplay_mapdata::archive::Archive::open(&out).unwrap();
        assert_eq!(archive.read("ui/dashboard/westplay_scania_2025_advanced_gps.sii").unwrap(), None);
        let interior = "def/vehicle/truck/scania.s_2016/interior/cab_s_high_smartdash_advanced_dark.sii";
        assert_eq!(archive.read(interior).unwrap(), None);
        drop(archive);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_coaches_gps_parts_screen_is_the_games_toy_gps_the_bus_mods_untouched() {
        let root = std::env::temp_dir().join(format!("westplay-mod-ob-gps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // The game, with the toy GPS's script (Cabin Accessories): its window 300 x 168.
        let game = root.join("game");
        std::fs::create_dir_all(game.join("ui/dashboard")).unwrap();
        let toy = "SiiNunit
{
ui::window : _nameless.wnd {
 my_children: 1
 my_children[0]: _nameless._.map
 coords_l: 0
 coords_r: 300
 coords_t: 168
 coords_b: 0
 my_parent: null
}

ui::text : _nameless._.map {
 my_parent: _nameless.wnd
}

}
";
        std::fs::write(game.join("ui/dashboard/gps.dlc_toys.sii"), toy).unwrap();
        let with_toy = GameFiles::new(vec![westplay_mapdata::files::Mount::Folder(game.clone())]);
        let script = "ui/dashboard/gps.dlc_toys.sii";
        let built_script = |game: &GameFiles| {
            let (bytes, built) = build(game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
            assert!(built.patched.contains(&"ob-gps"), "{built:?}");
            let out = root.join("westplay.scs");
            std::fs::write(&out, bytes).unwrap();
            let archive = westplay_mapdata::archive::Archive::open(&out).unwrap();
            // Nothing of the buses' mods: no part of theirs, no interior, no script of their own screens.
            for truck in ["setra516hd", "t16shd", "setradt531", "manlions"] {
                let dir = format!("def/vehicle/truck/{truck}");
                assert_eq!(archive.list(&dir).unwrap(), None, "{truck}");
            }
            for own in ["t16_gps.sii", "actros_gps.sii"] {
                assert_eq!(archive.read(&format!("ui/dashboard/{own}")).unwrap(), None);
            }
            assert_eq!(archive.read("ui/gps/man_e6_gps.sii").unwrap(), None);
            String::from_utf8(archive.read(script).unwrap().unwrap()).unwrap()
        };
        // WestPlay's image over the toy GPS's whole window (each bus's glass shows its own part of it:
        // `westplay_shared::page_place`), on top of its map: the frame's rows over it, its texture's 728 below it, each
        // edge a unit beyond.
        let patched = built_script(&with_toy);
        assert!(
            patched.contains(
                " my_children: 2
 my_children[0]: _nameless._.map
 my_children[1]: _nameless.westplay.screen
"
            ),
            "{patched}"
        );
        assert!(patched.contains("screen_ob_gps.mat"), "{patched}");
        assert!(patched.contains(" coords_l: -1
 coords_r: 301
 coords_t: 169
 coords_b: -3
"), "{patched}");
        // A player without Cabin Accessories: a window of WestPlay's at its path, its image alone.
        let own = built_script(&GameFiles::new(Vec::new()));
        assert!(own.contains(" coords_l: -1
 coords_r: 1001
 coords_t: 1001
 coords_b: -13
"), "{own}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// YTHQ Tablet's own screen scripts as its archive has them (shortened): a 256 x 256 window with the "screen on"
    /// group (id 100) and YTHQ's map in it, over the whole of it.
    const YTHQ_SCRIPT: &str = "SiiNunit\n{\nui::window : _nameless.025feab00 {\n my_children: 1\n my_children[0]: _nameless._.gpsbsd\n coords_l: 0\n coords_r: 256\n coords_t: 256\n coords_b: 0\n my_parent: null\n}\n\nui::group : _nameless._.gpsbsd {\n fitting: false\n my_children: 1\n my_children[0]: _nameless._.map\n coords_l: 0\n coords_r: 256\n coords_t: 256\n coords_b: 0\n id: 100\n layer: 2\n my_parent: _nameless.025feab00\n}\n\nui::text : _nameless._.map {\n text: \"<img src=/material/ui/dashboard/bare_map.mat xscale=stretch yscale=stretch>\"\n layer: 2\n my_parent: _nameless._.gpsbsd\n}\n\n}\n";

    /// A Workshop content folder with YTHQ Tablet's item in it (`3595920707/universal.scs`, a zip of `files`: its
    /// scripts and its vehicles' definitions).
    fn ythq_workshop(name: &str, kind: Kind, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("westplay-ythq-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let item = root.join(ythq_item(kind));
        std::fs::create_dir_all(&item).unwrap();
        let files: Vec<(String, Vec<u8>)> =
            files.iter().map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec())).collect();
        std::fs::write(item.join("universal.scs"), zip(&files)).unwrap();
        root
    }

    /// ETS2's and ATS's YTHQ Tablet are separate Workshop items with separate ids: each game's id has to find
    /// the mod in that game's Workshop folder, and look in the other one's item for nothing.
    #[test]
    fn finds_ythq_tablet_under_each_games_own_workshop_item() {
        for (kind, item) in [(Kind::Ets2, "3595920707"), (Kind::Ats, "3598886740")] {
            assert_eq!(ythq_item(kind), item);
            let other = match kind {
                Kind::Ets2 => Kind::Ats,
                Kind::Ats => Kind::Ets2,
            };
            let workshop = ythq_workshop("ids", kind, &[("ui/dashboard/ythq_tablet_gps.sii", YTHQ_SCRIPT)]);
            assert_eq!(ythq_archives(Some(&workshop), kind).len(), 1, "{item}");
            assert!(ythq_archives(Some(&workshop), other).is_empty(), "{item}");
            let _ = std::fs::remove_dir_all(&workshop);
        }
    }

    /// A tablet's definition as YTHQ's archive has them (one file a tablet, its `ui_path` the script its screen draws,
    /// `ui_drawable_size` and the rest as YTHQ writes them).
    fn ythq_accessory(unit: &str, ui_path: &str) -> String {
        format!(
            "SiiNunit\n{{\n\taccessory_addon_int_ui_data : {unit}\n\t{{\n\t\tname: \"YTHQ Tablet\"\n\t\tprice: 1\n\t\t\
             interior_model: \"/vehicle/truck/upgrade/interior_decors/toyac/volvo.fh_2021/ythq_tablet.pmd\"\n\t\t\
             unlock: 0\n\t\tpart_type: aftermarket\n\t\tui_path: \"{ui_path}\"\n\t\t\
             ui_drawable_texture_path: \"/vehicle/truck/share/gps.tobj\"\n\t\tui_drawable_size: (512, 512)\n\t}}\n}}\n"
        )
    }

    /// A tablet's model as YTHQ Tablet's archive has them (shortened): its materials' paths, its screen face's the
    /// trucks' own screens' texture object (`YTHQ_FACTORY_UI`), as all its tablets' models' are.
    const YTHQ_TABLET_PMD: &str = "\u{1}mdl vehicle/truck/upgrade/interior_decors/toyac/volvo.fh_2021/ythq_tablet\
         \u{0}/automat/ab/abcdef0123456789.mat\u{0}/automat/cd/fedcba9876543210.mat\u{0}";
    const YTHQ_TABLET_PMD_PATH: &str = "vehicle/truck/upgrade/interior_decors/toyac/volvo.fh_2021/ythq_tablet.pmd";
    /// Its screen face's material, sampling the trucks' own screens' texture object.
    const YTHQ_TABLET_SCREEN_MAT: &str = "automat/ab/abcdef0123456789.mat";
    const YTHQ_TABLET_SCREEN_MAT_TEXT: &str = "effect : \"eut2.dif.lum.spec.lvcol.rfx\" {\n\ttexture : \"texture_base\" {\n\t\tsource : \"/vehicle/truck/share/gps.tobj\"\n\t}\n}\n";
    /// Its case's material: another texture, left as it is.
    const YTHQ_TABLET_CASE_MAT: &str = "automat/cd/fedcba9876543210.mat";
    const YTHQ_TABLET_CASE_MAT_TEXT: &str = "effect : \"eut2.dif.spec.rfx\" {\n\ttexture : \"texture_base\" {\n\t\tsource : \"/vehicle/truck/upgrade/interior_decors/toyac/ythq_tablet_case.tobj\"\n\t}\n}\n";
    /// Its tablets' own UI texture object, which exists in its archive: the UI their screens' scripts have is drawn
    /// into it.
    const YTHQ_TABLET_UI_PATH: &str = "vehicle/truck/upgrade/interior_decors/toyac/ythq_tablet_ui.tobj";

    /// The files of YTHQ Tablet's that its tablets' models need: the model, its materials, and its tablets' own UI
    /// texture object.
    fn ythq_tablet_files() -> Vec<(&'static str, String)> {
        vec![
            (YTHQ_TABLET_PMD_PATH, YTHQ_TABLET_PMD.to_owned()),
            (YTHQ_TABLET_SCREEN_MAT, YTHQ_TABLET_SCREEN_MAT_TEXT.to_owned()),
            (YTHQ_TABLET_CASE_MAT, YTHQ_TABLET_CASE_MAT_TEXT.to_owned()),
            (YTHQ_TABLET_UI_PATH, "tobj".to_owned()),
        ]
    }

    /// The Volvo FH's and the Renault T 2024's mirror scripts, and tablets on a Volvo FH's dash and windscreen (each
    /// slot: `toyac`, `set_glass`) and a Renault T 2024's, with one accessory that isn't YTHQ's tablets' of a script of
    /// WestPlay's copies.
    fn ythq_workshop_files(script: &str) -> Vec<(&'static str, String)> {
        let nfb = "/ui/dashboard/ythq_tablet_gps_mirror_nfb.sii";
        let mut files: Vec<(&'static str, String)> = vec![
            ("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii", script.to_owned()),
            ("ui/dashboard/ythq_renault_t_2024_gps.sii", script.to_owned()),
            (
                "def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii",
                ythq_accessory("ythq.larger.tablet.volvo.fh_2021.toyac", nfb),
            ),
            (
                "def/vehicle/truck/volvo.fh_2021/accessory/set_glass/ythq_larger_tablet_uk.sii",
                ythq_accessory("ythq.larger.tablet.uk.volvo.fh_2021.toyglass", nfb),
            ),
            (
                "def/vehicle/truck/renault.t/accessory/toyac/ythq_renault_tablet.sii",
                ythq_accessory("ythq.larger.tablet.renault.t.toyac", "/ui/dashboard/ythq_renault_t_2024_gps.sii"),
            ),
            (
                "def/vehicle/truck/volvo.fh_2021/accessory/toyac/another.sii",
                ythq_accessory("another.accessory.toyac", "/ui/dashboard/def/not/one/of/westplays.sii"),
            ),
        ];
        files.extend(ythq_tablet_files());
        files
    }

    /// The files' texts as paths, for a test's fixture: (`&str` path, `&str` text).
    fn as_files<'a>(files: &'a [(&'a str, String)]) -> Vec<(&'a str, &'a str)> {
        files.iter().map(|(path, text)| (*path, text.as_str())).collect()
    }

    /// American Truck Simulator's YTHQ Tablet's one screen script, shortened (its window its displays' whole surface:
    /// 1024 x 1024; the "screen on" group id 100 and YTHQ's mirror beside it).
    const ATS_YTHQ_SCRIPT_PATH: &str = "ui/dashboard/ythq_tablet_gps_mirror_h.sii";
    const ATS_YTHQ_SCRIPT: &str = "SiiNunit\n{\n\tui::window : _nameless.025feab00 {\n\t\tmy_children: 2\n\t\t\
        my_children[0]: _nameless._.gpsbsd\n\t\tmy_children[1]: _nameless._.shared\n\t\tcoords_l: 0\n\t\tcoords_r: 1024\n\t\t\
        coords_t: 1024\n\t\tcoords_b: 0\n\t\tmy_parent: null\n\t}\n\n\tui::group : _nameless._.shared {\n\t\t\
        my_children: 0\n\t\tcoords_l: 0\n\t\tcoords_r: 1024\n\t\tcoords_t: 1024\n\t\tcoords_b: 0\n\t\tid: 950\n\t\t\
        layer: 1\n\t\tmy_parent: _nameless.025feab00\n\t}\n\n\tui::group : _nameless._.gpsbsd {\n\t\tmy_children: 0\n\t\t\
        coords_l: 0\n\t\tcoords_r: 1024\n\t\tcoords_t: 1024\n\t\tcoords_b: 0\n\t\tid: 100\n\t\tlayer: 2\n\t\t\
        my_parent: _nameless.025feab00\n\t}\n\n}\n";
    /// Its tablets' model (shortened): its screen face's material the trucks' own screens' texture object (the one the
    /// add-ons also replace), its case's another.
    const ATS_YTHQ_PMD_PATH: &str =
        "vehicle/truck/upgrade/interior_decors/toyac/volvo.vnl2025tr/ythq_tablettb.pmd";
    const ATS_YTHQ_PMD: &str = "\u{1}mdl vehicle/truck/upgrade/interior_decors/toyac/volvo.vnl2025tr/ythq_tablettb\
         \u{0}/automat/5d/5d8cd8f82a29c50f.mat\u{0}/automat/2b/2ba00d35b06091aa.mat\u{0}";
    const ATS_YTHQ_SCREEN_MAT: &str = "automat/5d/5d8cd8f82a29c50f.mat";
    const ATS_YTHQ_CASE_MAT: &str = "automat/2b/2ba00d35b06091aa.mat";
    const ATS_YTHQ_DEFINITION: &str = "def/vehicle/truck/volvo.vnl2025/accessory/toyac/ythq_larger_tablet.sii";

    /// A tablet's definition as American Truck Simulator's YTHQ Tablet has them: its one screen script as its
    /// `ui_path`, the trucks' own screens' texture object, and a 1024 x 1024 UI (`ui_drawable_size`).
    fn ats_ythq_accessory() -> String {
        format!(
            "SiiNunit\n{{\n\taccessory_addon_int_ui_data : ythq.larger.tablet.volvo.vnl2025.toyac\n\t{{\n\t\t\
             name: \"YTHQ Tablet\"\n\t\tprice: 1\n\t\tinterior_model: \"/{ATS_YTHQ_PMD_PATH}\"\n\t\t\
             exterior_model: \"/{ATS_YTHQ_PMD_PATH}\"\n\t\tunlock: 0\n\t\ticon: \"larger_tablet_gps_icon1\"\n\t\t\
             part_type: aftermarket\n\t\tui_path: \"/{ATS_YTHQ_SCRIPT_PATH}\"\n\t\t\
             ui_drawable_texture_path: \"/{YTHQ_FACTORY_UI}\"\n\t\tui_drawable_size: (1024, 1024)\n\t}}\n}}\n"
        )
    }

    #[test]
    fn ats_ythq_tablets_keep_their_own_screens_and_definitions_westplays_tablet_picture_over_them() {
        // Its one script for every truck's factory screen: not ETS2's names, which its files haven't.
        assert_eq!(ythq_screens(Kind::Ats), &[(ATS_YTHQ_SCRIPT_PATH, "generic")]);
        assert!(ythq_screens(Kind::Ats).iter().all(|(script, _)| !script.contains("nfb")));
        // Its window is its displays' whole surface: the truck screen's picture goes on all of it, WestPlay's tablet
        // picture on the band its tablets' screens show ([`YTHQ_TABLET_SHOWN`]).
        assert_eq!(ythq_shown(Kind::Ats, "generic"), None);
        assert!(ythq_shown(Kind::Ets2, "volvo-fh").is_some());
        let files = [
            (ATS_YTHQ_SCRIPT_PATH, ATS_YTHQ_SCRIPT.to_owned()),
            (ATS_YTHQ_DEFINITION, ats_ythq_accessory()),
            (ATS_YTHQ_PMD_PATH, ATS_YTHQ_PMD.to_owned()),
            (
                ATS_YTHQ_SCREEN_MAT,
                format!("effect : \"eut2.dif.lum.spec.lvcol.rfx\" {{\n\ttexture : \"texture_base\" {{\n\t\tsource : \"/{YTHQ_FACTORY_UI}\"\n\t}}\n}}\n"),
            ),
            (
                ATS_YTHQ_CASE_MAT,
                format!("effect : \"eut2.dif.spec.rfx\" {{\n\ttexture : \"texture_base\" {{\n\t\tsource : \"/vehicle/truck/upgrade/interior_decors/toyac/ythq_tablet_case.tobj\"\n\t}}\n}}\n"),
            ),
            (YTHQ_TABLET_UI_PATH, "tobj".to_owned()),
        ];
        let workshop = ythq_workshop("ats", Kind::Ats, &as_files(&files));
        let game = GameFiles::new(Vec::new());
        let (bytes, built) =
            build(&game, &[], &cab_mark::Marks::default(), Kind::Ats, Some(workshop.as_path())).unwrap();
        // Its own one script, patched; none of its tablets' definitions and copies (it keeps its own: its `ui_path` the
        // script the mod patches, its `ui_drawable_texture_path` the trucks' own screens' texture).
        assert_eq!(built.ythq, [ATS_YTHQ_SCRIPT_PATH]);
        assert!(built.ythq_tablets.is_empty(), "{:?}", built.ythq_tablets);
        assert!(built.ythq_accessories.is_empty(), "{:?}", built.ythq_accessories);
        assert_eq!(built.ythq_tablet_materials, [ATS_YTHQ_SCREEN_MAT]);
        assert!(built.ythq_problem.is_none(), "{:?}", built.ythq_problem);
        let out = workshop.join("westplay.scs");
        std::fs::write(&out, &bytes).unwrap();
        let archive = Archive::open(&out).unwrap();
        let text = |path: &str| String::from_utf8(archive.read(path).unwrap().unwrap()).unwrap();
        // Its one script: WestPlay's generic picture over its window whole (the trucks' screens keep YTHQ's, theirs
        // not being WestPlay's), and WestPlay's tablet picture on the band its tablets' screens show, under the
        // screen's picture and over YTHQ's own elements.
        let screen = text(ATS_YTHQ_SCRIPT_PATH);
        assert!(screen.contains("screen_generic.mat"), "{screen}");
        assert!(screen.contains(" coords_l: 0\n coords_r: 1024\n coords_t: 1024\n coords_b: 0\n"), "{screen}");
        let marker = format!("ui::text : {TABLET_OVERLAY} {{");
        let tablet = screen.split(&marker).nth(1).unwrap_or_else(|| panic!("{screen}"));
        assert!(tablet.contains("<img src=/material/ui/westplay/screen_tablet.mat xscale=stretch yscale=stretch>"));
        assert!(tablet.contains(&format!(" layer: {TABLET_LAYER}\n")), "{tablet}");
        // The band its own tablet models show: v 0.1922..0.8078 of its 1024 x 1024 window.
        assert!(tablet.contains(" coords_l: 0\n coords_r: 1024\n coords_t: 827\n coords_b: 197\n"), "{tablet}");
        // YTHQ's own elements stay: its window and its groups, as its archive has them.
        assert!(screen.contains("ui::window : _nameless.025feab00 {"), "{screen}");
        assert!(screen.contains("ui::group : _nameless._.shared {"), "{screen}");
        assert!(screen.contains("ui::group : _nameless._.gpsbsd {"), "{screen}");
        assert!(screen.contains("id: 100"), "{screen}");
        // Its tablets' definitions: YTHQ's own (the mod carries none), its UI addon loading what it always did.
        assert!(archive.read(ATS_YTHQ_DEFINITION).unwrap().is_none(), "its definitions carried");
        // Its screen material at a path of the mod's own (YTHQ's own path stays WestPlay's tablet texture, which the
        // Ford cars' of `ats-ythq-ford` sample), sampling the trucks' own screens' texture as YTHQ has it, and its
        // model's descriptor naming it (the same length, changed in place).
        let isolated = ythq_tablet_material_path(ATS_YTHQ_SCREEN_MAT);
        assert_ne!(isolated, ATS_YTHQ_SCREEN_MAT);
        let screen_mat = text(&isolated);
        assert!(screen_mat.contains(&format!("\"/{YTHQ_FACTORY_UI}\"")), "{screen_mat}");
        assert!(!screen_mat.contains(&format!("\"/{YTHQ_TABLET_UI}\"")), "{screen_mat}");
        let shared = text(ATS_YTHQ_SCREEN_MAT);
        assert!(shared.contains("screen_tablet.tobj"), "YTHQ's own path not the add-ons': {shared}");
        let model = text(ATS_YTHQ_PMD_PATH);
        assert!(model.contains(&isolated), "{model}");
        assert!(!model.contains(ATS_YTHQ_SCREEN_MAT), "{model}");
        assert!(model.contains(ATS_YTHQ_CASE_MAT), "{model}");
        assert!(archive.read(ATS_YTHQ_CASE_MAT).unwrap().is_none(), "a material of its own carried untouched");
        // Its tablets' models the add-ons' (their screen faces laid out for WestPlay's tablet texture): left out, its
        // own used, which show the band of the trucks' own screens' texture their screens were made for.
        for path in [
            "vehicle/truck/upgrade/interior_decors/toyac/volvo.vnl2025tr/ythq_tablettb.pmg",
            "vehicle/truck/upgrade/interior_decors/toyac/volvo.vnl2025tr/ythq_tablet_smalltb.pmg",
        ] {
            assert!(archive.read(path).unwrap().is_none(), "{path} carried");
        }
        drop(archive);
        // As its archive has it now: current (the app needn't build it again).
        assert!(ythq_screens_current(Kind::Ats, Some(workshop.as_path()), &out));
        let _ = std::fs::remove_dir_all(&workshop);
    }

    #[test]
    fn ythq_tablets_screens_get_westplays_tablet_picture_too() {
        let volvo = screen_layouts().iter().position(|layout| layout.id == "volvo-fh").unwrap();
        let rows = ScreenLayout::texture_height(volvo);
        let screen = patch_gps_script(YTHQ_SCRIPT, "screen_volvo_fh", None, rows).unwrap();
        let patched = patch_ythq_tablet_script(&screen, "screen_tablet", None).unwrap();
        // The tablet picture: the tablet layout's texture, over the window whole (its own coordinates), a child of the
        // window (the truck screen's a child of the "screen on" group).
        let marker = format!("ui::text : {TABLET_OVERLAY} {{");
        assert!(patched.contains(&marker), "{patched}");
        let tablet = patched.split(&marker).nth(1).unwrap();
        assert!(tablet.contains("<img src=/material/ui/westplay/screen_tablet.mat xscale=stretch yscale=stretch>"));
        assert!(tablet.contains(" coords_l: 0\n coords_r: 256\n coords_t: 256\n coords_b: 0\n"), "{tablet}");
        assert!(tablet.contains(&format!(" layer: {TABLET_LAYER}\n")), "{tablet}");
        assert!(tablet.contains(" my_parent: _nameless.025feab00\n"), "{tablet}");
        assert!(
            patched.contains(
                " my_children: 2\n my_children[0]: _nameless._.gpsbsd\n my_children[1]: _nameless.westplay.tablet\n"
            ),
            "{patched}"
        );
        // Under the truck screen's picture: both over YTHQ's own (its highest layer is 16).
        assert!(TABLET_LAYER < SCREEN_LAYER && SCREEN_LAYER > 16);
        // Twice: said, not made twice; a script with no window to put it in: said.
        assert!(patch_ythq_tablet_script(&patched, "screen_tablet", None).is_err());
        assert!(patch_ythq_tablet_script("SiiNunit\n{\n}\n", "screen_tablet", None).is_err());
        // On the part of the window a tablet's screen shows ([`YTHQ_TABLET_SHOWN`], American Truck Simulator's one
        // script): the middle of the window's 256 units, its width whole.
        let placed = patch_ythq_tablet_script(&screen, "screen_tablet", Some(&YTHQ_TABLET_SHOWN)).unwrap();
        let tablet = placed.split(&marker).nth(1).unwrap();
        assert!(tablet.contains(" coords_l: 0\n coords_r: 256\n coords_t: 207\n coords_b: 49\n"), "{tablet}");
    }

    #[test]
    fn ythq_tablets_get_screens_of_their_own_and_their_definitions_point_at_them() {
        // Its scripts for the Volvo FH's and the Renault T 2024's factory screens, and its tablets of both.
        let files = ythq_workshop_files(YTHQ_SCRIPT);
        let workshop = ythq_workshop("patched", Kind::Ets2, &as_files(&files));
        let game = GameFiles::new(Vec::new());
        let out = workshop.join("westplay.scs");
        let make = || {
            let (bytes, built) =
                build(&game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, Some(workshop.as_path()))
                    .unwrap();
            std::fs::write(&out, &bytes).unwrap();
            built
        };
        let built = make();
        assert_eq!(
            built.ythq,
            ["ui/dashboard/ythq_tablet_gps_mirror_nfb.sii", "ui/dashboard/ythq_renault_t_2024_gps.sii"]
        );
        // A copy for each of the two, and its tablets' definitions pointed at them (the accessory that isn't its
        // tablets' of them left out).
        assert_eq!(
            built.ythq_tablets,
            [
                "ui/dashboard/westplay_ythq/ythq_renault_t_2024_gps.sii",
                "ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_nfb.sii"
            ]
        );
        assert_eq!(
            built.ythq_accessories,
            [
                "def/vehicle/truck/renault.t/accessory/toyac/ythq_renault_tablet.sii",
                "def/vehicle/truck/volvo.fh_2021/accessory/set_glass/ythq_larger_tablet_uk.sii",
                "def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii"
            ]
        );
        // Its tablets' models' screen material, pointed at its tablets' own UI texture (its case's left as it is).
        assert_eq!(built.ythq_tablet_materials, [YTHQ_TABLET_SCREEN_MAT]);
        // The rest of its screens aren't in this shortened archive: said, not guessed.
        assert!(
            built.ythq_problem.as_deref().is_some_and(|problem| problem.contains("not in YTHQ Tablet's files")),
            "{:?}",
            built.ythq_problem
        );
        let archive = Archive::open(&out).unwrap();
        let text = |path: &str| String::from_utf8(archive.read(path).unwrap().unwrap()).unwrap();
        // The factory screen's script: WestPlay's texture of its layout, on the part its display shows (0.54-0.55% of
        // each edge), over YTHQ's map, which stays under it (the texture is transparent while WestPlay doesn't draw);
        // and none of the tablet's picture, which the tablets draw from a copy of their own.
        let volvo = text("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii");
        assert!(volvo.contains("screen_volvo_fh.mat xscale=stretch yscale=stretch"), "{volvo}");
        assert!(volvo.contains(" coords_l: 1\n coords_r: 255\n coords_t: 255\n coords_b: -1\n"), "{volvo}");
        assert!(volvo.contains("/material/ui/dashboard/bare_map.mat"), "{volvo}");
        assert!(volvo.contains(" layer: 100\n"), "{volvo}");
        assert!(!volvo.contains("screen_tablet.mat"), "{volvo}");
        assert!(!volvo.contains(TABLET_OVERLAY), "{volvo}");
        // The tablet's copy: the tablet layout's texture on the part of the window YTHQ's tablet screens show (its own
        // frame's shape), under where the screen's picture would be (YTHQ's own highest layer is 16), and none of the
        // truck screen's.
        let tablet = text("ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_nfb.sii");
        assert!(tablet.contains(&format!("ui::text : {TABLET_OVERLAY} {{")), "{tablet}");
        assert!(tablet.contains("<img src=/material/ui/westplay/screen_tablet.mat xscale=stretch yscale=stretch>"));
        assert!(tablet.contains(&format!(" layer: {TABLET_LAYER}\n")), "{tablet}");
        assert!(
            tablet.contains(
                " coords_l: 0\n coords_r: 256\n coords_t: 207\n coords_b: 49\n area_l: 1\n area_r: 0\n area_t: 0\n \
                 area_b: 1\n id: 0\n"
            ),
            "{tablet}"
        );
        assert!(!tablet.contains("_nameless.westplay.screen"), "{tablet}");
        assert!(!tablet.contains("screen_volvo_fh.mat"), "{tablet}");
        assert!(tablet.contains("/material/ui/dashboard/bare_map.mat"), "{tablet}");
        // Its tablets' definitions: at its own paths, their `ui_path`s the copies' and their
        // `ui_drawable_texture_path`s its tablets' own UI texture (the one their models' screen materials sample
        // too), every other field as YTHQ wrote them (`ui_drawable_size` included).
        let dash = text("def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii");
        assert!(dash.contains("ui_path: \"/ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_nfb.sii\""), "{dash}");
        assert!(
            dash.contains(&format!("ui_drawable_texture_path: \"/{YTHQ_TABLET_UI}\"")),
            "{dash}"
        );
        assert!(dash.contains("ui_drawable_size: (512, 512)"), "{dash}");
        assert!(
            dash.contains("interior_model: \"/vehicle/truck/upgrade/interior_decors/toyac/volvo.fh_2021/"),
            "{dash}"
        );
        assert!(dash.contains("accessory_addon_int_ui_data : ythq.larger.tablet.volvo.fh_2021.toyac"), "{dash}");
        let glass = text("def/vehicle/truck/volvo.fh_2021/accessory/set_glass/ythq_larger_tablet_uk.sii");
        assert!(glass.contains("/ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_nfb.sii"), "{glass}");
        // The model's screen material: the trucks' own screens' texture object no more, its tablets' own instead;
        // its case's material as YTHQ has it.
        let screen = text(YTHQ_TABLET_SCREEN_MAT);
        assert!(screen.contains(&format!("\"/{YTHQ_TABLET_UI}\"")), "{screen}");
        assert!(!screen.contains(&format!("\"/{YTHQ_FACTORY_UI}\"")), "{screen}");
        assert!(archive.read(YTHQ_TABLET_CASE_MAT).unwrap().is_none(), "a material of its own carried untouched");
        // Not one of its tablets': nothing of it in the mod.
        assert!(
            archive.read("def/vehicle/truck/volvo.fh_2021/accessory/toyac/another.sii").unwrap().is_none(),
            "an accessory of no tablet of YTHQ's carried"
        );
        drop(archive);
        // As its archive has them now: current. Updated: not, and made again current.
        assert!(ythq_screens_current(Kind::Ets2, Some(&workshop), &out));
        let changed = YTHQ_SCRIPT.replace("bare_map.mat", "bare_map2.mat");
        let updated: Vec<(&str, String)> = files
            .iter()
            .map(|(path, text)| (*path, if *text == YTHQ_SCRIPT { changed.clone() } else { text.clone() }))
            .collect();
        let workshop_files = updated.iter().map(|(path, text)| (*path, text.as_str())).collect::<Vec<_>>();
        let files: Vec<(String, Vec<u8>)> =
            workshop_files.iter().map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec())).collect();
        std::fs::write(workshop.join(ythq_item(Kind::Ets2)).join("universal.scs"), zip(&files)).unwrap();
        assert!(!ythq_screens_current(Kind::Ets2, Some(&workshop), &out));
        make();
        assert!(ythq_screens_current(Kind::Ets2, Some(&workshop), &out));
        let _ = std::fs::remove_dir_all(&workshop);
    }

    #[test]
    fn ythq_tablets_own_scripts_outside_the_factory_screens_are_copied_too() {
        // Its tablets of the old trucks' and the coaches' (`ythq_tablet_gps.sii`, `ythq_tablet_gps_mirror_bus.sii`):
        // the coach's factory screen names the bus one too, and the mod's add-ons carry it there.
        let mut files: Vec<(&'static str, String)> = vec![
            ("ui/dashboard/ythq_tablet_gps.sii", YTHQ_SCRIPT.to_owned()),
            ("ui/dashboard/ythq_tablet_gps_mirror_bus.sii", YTHQ_SCRIPT.to_owned()),
            (
                "def/vehicle/truck/scania.g/accessory/toyac/ythq_larger_tablet.sii",
                ythq_accessory("ythq.larger.tablet.scania.g.toyac", "/ui/dashboard/ythq_tablet_gps.sii"),
            ),
            (
                "def/vehicle/bus/man.lions_24/accessory/toyac/ythq_larger_tablet.sii",
                ythq_accessory("ythq.larger.tablet.man.lions_24.toyac", "/ui/dashboard/ythq_tablet_gps_mirror_bus.sii"),
            ),
        ];
        files.extend(ythq_tablet_files());
        let workshop = ythq_workshop("tablets-only", Kind::Ets2, &as_files(&files));
        let game = GameFiles::new(Vec::new());
        let (bytes, built) =
            build(&game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, Some(workshop.as_path())).unwrap();
        // No factory screen of its is in this shortened archive: nothing patched.
        assert!(built.ythq.is_empty(), "{:?}", built.ythq);
        assert_eq!(
            built.ythq_tablets,
            [
                "ui/dashboard/westplay_ythq/ythq_tablet_gps.sii",
                "ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_bus.sii"
            ]
        );
        assert_eq!(built.ythq_accessories.len(), 2, "{:?}", built.ythq_accessories);
        let out = workshop.join("westplay.scs");
        std::fs::write(&out, &bytes).unwrap();
        let archive = Archive::open(&out).unwrap();
        let text = |path: &str| String::from_utf8(archive.read(path).unwrap().unwrap()).unwrap();
        // The copies: the tablet's picture alone.
        for path in &built.ythq_tablets {
            let script = text(path);
            assert!(script.contains("screen_tablet.mat"), "{path}");
            assert!(script.contains(TABLET_OVERLAY), "{path}");
            assert!(!script.contains("_nameless.westplay.screen"), "{path}");
        }
        // The bus script at its own path: the add-ons' copy isn't in the mod (YTHQ's own there, no WestPlay picture on
        // the factory screen that still loads it), now that its tablets draw copies of their own.
        assert!(
            archive.read("ui/dashboard/ythq_tablet_gps_mirror_bus.sii").unwrap().is_none(),
            "the add-ons' copy of the bus script carried"
        );
        // The tablets' definitions, pointed at the copies and at its tablets' own UI texture.
        let old = text("def/vehicle/truck/scania.g/accessory/toyac/ythq_larger_tablet.sii");
        assert!(old.contains("ui_path: \"/ui/dashboard/westplay_ythq/ythq_tablet_gps.sii\""), "{old}");
        assert!(old.contains(&format!("ui_drawable_texture_path: \"/{YTHQ_TABLET_UI}\"")), "{old}");
        let coach = text("def/vehicle/bus/man.lions_24/accessory/toyac/ythq_larger_tablet.sii");
        assert!(coach.contains("ui_path: \"/ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_bus.sii\""), "{coach}");
        assert!(coach.contains(&format!("ui_drawable_texture_path: \"/{YTHQ_TABLET_UI}\"")), "{coach}");
        let _ = std::fs::remove_dir_all(&workshop);
    }

    #[test]
    fn ythq_tablets_definitions_not_as_read_leave_the_tablet_picture_on_the_factory_screens() {
        // A definition with its `ui_path` not quoted (a YTHQ version written otherwise): its tablets' scripts can't be
        // pointed at copies, so they keep WestPlay's tablet picture on the factory screens' scripts, as before them.
        let tablet = "/ui/dashboard/ythq_tablet_gps_mirror_nfb.sii";
        let unquoted = ythq_accessory("ythq.larger.tablet.volvo.fh_2021.toyac", tablet)
            .replace(&format!("\"{tablet}\""), tablet);
        let files = vec![
            ("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii", YTHQ_SCRIPT.to_owned()),
            ("def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii", unquoted),
        ];
        let workshop = ythq_workshop("unquoted", Kind::Ets2, &as_files(&files));
        let game = GameFiles::new(Vec::new());
        let (bytes, built) =
            build(&game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, Some(workshop.as_path())).unwrap();
        assert!(
            built.ythq_tablets.is_empty()
                && built.ythq_accessories.is_empty()
                && built.ythq_tablet_materials.is_empty()
        );
        let problem = built.ythq_problem.as_deref().unwrap_or_default();
        assert!(problem.contains("not a quoted path"), "{problem}");
        let out = workshop.join("westplay.scs");
        std::fs::write(&out, &bytes).unwrap();
        let archive = Archive::open(&out).unwrap();
        let script =
            String::from_utf8(archive.read("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii").unwrap().unwrap()).unwrap();
        // Both pictures on the factory screen's script: the truck screen's and the tablet's (which its tablet draws).
        assert!(script.contains("screen_volvo_fh.mat"), "{script}");
        assert!(script.contains("screen_tablet.mat"), "{script}");
        assert!(script.contains("_nameless.westplay.screen") && script.contains(TABLET_OVERLAY), "{script}");
        // Its definition as YTHQ has it (the mod's own copy of the path: the add-ons' one, if it has it).
        let definition = "def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii";
        assert!(archive.read(definition).unwrap().is_none());
        drop(archive);
        let _ = std::fs::remove_dir_all(&workshop);
    }

    #[test]
    fn ythq_tablets_screens_not_as_read_leave_the_mods_factory_screens_as_the_add_ons_have_it() {
        // Its one script here without the "screen on" group (a YTHQ version laid out otherwise): no factory screen of
        // its patched; its tablet's copy made all the same (no truck screen's picture goes on it to begin with).
        let bad = YTHQ_SCRIPT.replace("id: 100", "id: 7");
        let workshop = ythq_workshop("unread", Kind::Ets2, &[("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii", &bad)]);
        let game = GameFiles::new(Vec::new());
        let (bytes, built) =
            build(&game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, Some(workshop.as_path()))
                .unwrap();
        assert!(built.ythq.is_empty());
        let problem = built.ythq_problem.as_deref().unwrap_or_default();
        assert!(problem.contains("ythq_tablet_gps_mirror_nfb.sii: no group with id 100"), "{problem}");
        assert_eq!(built.ythq_tablets, ["ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_nfb.sii"]);
        let out = workshop.join("westplay.scs");
        std::fs::write(&out, &bytes).unwrap();
        // The add-ons' copy stays as it was (YTHQ's old script: the tablet texture on the factory screen).
        let archive = Archive::open(&out).unwrap();
        let script =
            String::from_utf8(archive.read("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii").unwrap().unwrap()).unwrap();
        assert!(script.contains("screen_tablet.mat"), "{script}");
        drop(archive);
        // Its tablet's copy in the mod: current.
        assert!(ythq_screens_current(Kind::Ets2, Some(&workshop), &out));
        let _ = std::fs::remove_dir_all(&workshop);
    }

    #[test]
    fn a_tablet_models_materials_are_found_and_its_screen_pointed_at_its_tablets_own_texture() {
        // The model's material paths as its descriptor has them.
        assert_eq!(
            pmd_materials(YTHQ_TABLET_PMD.as_bytes()),
            [format!("/{YTHQ_TABLET_SCREEN_MAT}"), format!("/{YTHQ_TABLET_CASE_MAT}")]
        );
        assert_eq!(pmd_materials(b"nothing of a model"), Vec::<String>::new());
        // A definition's model and the texture object its tablet's screen draws into.
        let definition = ythq_accessory("a.tablet", "/ui/dashboard/ythq_tablet_gps_nfb.sii");
        assert_eq!(
            ythq_model(&definition).as_deref(),
            Some("/vehicle/truck/upgrade/interior_decors/toyac/volvo.fh_2021/ythq_tablet.pmd")
        );
        assert_eq!(ythq_texture_path(&definition).as_deref(), Some("/vehicle/truck/share/gps.tobj"));
        assert_eq!(ythq_texture_path("SiiNunit\n{\n}\n"), None);
        // Pointed at its tablets' own UI texture: its one line changed, the rest as it was.
        let (from, to) = (format!("/{YTHQ_FACTORY_UI}"), format!("/{YTHQ_TABLET_UI}"));
        let changed = repoint_quoted(&definition, "ui_drawable_texture_path", &from, &to).unwrap();
        assert!(changed.contains(&format!("ui_drawable_texture_path: \"{to}\"")), "{changed}");
        assert_eq!(changed.lines().count(), definition.lines().count());
        assert_eq!(changed.lines().filter(|line| line.trim_start().starts_with("ui_path:")).count(), 1, "{changed}");
        // Not this texture object: none.
        assert_eq!(repoint_quoted(&definition, "ui_drawable_texture_path", "/other.tobj", &to), None);
    }

    #[test]
    fn without_ythq_tablets_own_texture_its_definitions_keep_the_factory_screens_texture() {
        // Its archive without the texture object its tablets' screens draw into (another version): its tablets stay
        // on the trucks' own screens' texture object, and the build says why.
        let mut files: Vec<(&'static str, String)> = vec![
            ("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii", YTHQ_SCRIPT.to_owned()),
            (
                "def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii",
                ythq_accessory("ythq.larger.tablet.volvo.fh_2021.toyac", "/ui/dashboard/ythq_tablet_gps_mirror_nfb.sii"),
            ),
        ];
        files.extend(ythq_tablet_files().into_iter().filter(|(path, _)| *path != YTHQ_TABLET_UI_PATH));
        let workshop = ythq_workshop("no-tablet-ui", Kind::Ets2, &as_files(&files));
        let game = GameFiles::new(Vec::new());
        let (bytes, built) =
            build(&game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, Some(workshop.as_path())).unwrap();
        assert!(built.ythq_tablet_materials.is_empty(), "{:?}", built.ythq_tablet_materials);
        let problem = built.ythq_problem.as_deref().unwrap_or_default();
        assert!(problem.contains("its tablets' own screen texture"), "{problem}");
        let out = workshop.join("westplay.scs");
        std::fs::write(&out, &bytes).unwrap();
        let archive = Archive::open(&out).unwrap();
        let dash = String::from_utf8(
            archive.read("def/vehicle/truck/volvo.fh_2021/accessory/toyac/ythq_larger_tablet.sii").unwrap().unwrap(),
        )
        .unwrap();
        assert!(dash.contains(&format!("ui_drawable_texture_path: \"/{YTHQ_FACTORY_UI}\"")), "{dash}");
        assert!(dash.contains("/ui/dashboard/westplay_ythq/ythq_tablet_gps_mirror_nfb.sii"), "{dash}");
        let _ = std::fs::remove_dir_all(&workshop);
    }

    #[test]
    fn a_definitions_ui_path_is_found_and_changed() {
        let definition = ythq_accessory("a.tablet", "/ui/dashboard/ythq_tablet_gps_nfb.sii");
        assert_eq!(ythq_ui_paths(&definition).unwrap(), ["/ui/dashboard/ythq_tablet_gps_nfb.sii"]);
        let changed = repoint_ui_path(&definition, "/ui/dashboard/ythq_tablet_gps_nfb.sii", "/ui/dashboard/w/x.sii");
        let changed = changed.unwrap();
        assert!(changed.contains("ui_path: \"/ui/dashboard/w/x.sii\""), "{changed}");
        // Everything else as it was: the same lines but that one.
        let before: Vec<&str> = definition.lines().collect();
        let after: Vec<&str> = changed.lines().collect();
        assert_eq!(before.len(), after.len());
        for (before, after) in before.iter().zip(after.iter()) {
            if before.trim_start().starts_with("ui_path") {
                assert_eq!(*after, "\t\tui_path: \"/ui/dashboard/w/x.sii\"");
            } else {
                assert_eq!(before, after);
            }
        }
        // Not quoted: said; none: none; no such ui_path to change: none; a definition of two tablets': both found.
        assert!(ythq_ui_paths("ui_path: unquoted\n").is_err());
        assert_eq!(ythq_ui_paths("ui_drawable_size: (512, 512)\n").unwrap(), Vec::<String>::new());
        assert_eq!(repoint_ui_path("ui_path: \"other\"\n", "\"a\"", "\"b\""), None);
        assert_eq!(ythq_ui_paths("ui_path: \"a\"\nui_path: \"b\"\n").unwrap().len(), 2);
    }

    #[test]
    fn without_ythq_tablet_the_mods_screens_are_the_add_ons() {
        let game = GameFiles::new(Vec::new());
        let (bytes, built) = build(&game, &[], &cab_mark::Marks::default(), crate::game::Kind::Ets2, None).unwrap();
        assert!(built.ythq.is_empty() && built.ythq_problem.is_none());
        assert!(built.ythq_tablet_materials.is_empty(), "{:?}", built.ythq_tablet_materials);
        let out = std::env::temp_dir().join(format!("westplay-no-ythq-{}.scs", std::process::id()));
        std::fs::write(&out, &bytes).unwrap();
        let archive = Archive::open(&out).unwrap();
        let script =
            String::from_utf8(archive.read("ui/dashboard/ythq_tablet_gps_mirror_nfb.sii").unwrap().unwrap()).unwrap();
        assert!(script.contains("screen_tablet.mat"), "{script}");
        drop(archive);
        assert!(ythq_screens_current(Kind::Ets2, None, &out));
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn its_zip_reads_back() {
        let files = vec![("a/b.txt".to_owned(), b"hello".to_vec()), ("c.txt".to_owned(), vec![7u8; 5000])];
        let path = std::env::temp_dir().join(format!("westplay-mod-zip-{}.scs", std::process::id()));
        std::fs::write(&path, zip(&files)).unwrap();
        let archive = westplay_mapdata::archive::Archive::open(&path).unwrap();
        assert_eq!(archive.read("a/b.txt").unwrap().as_deref(), Some(&b"hello"[..]));
        assert_eq!(archive.read("c.txt").unwrap(), Some(vec![7u8; 5000]));
        drop(archive);
        let _ = std::fs::remove_file(path);
    }
}
