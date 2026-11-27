//! Bilibili: the real Bilibili.com in a WebView of its own, hidden, as the engine that plays the video. The
//! player never sees a Bilibili page: the cabin page has the Bilibili app (search, results, its own player
//! controls); this WebView's picture shows under the cabin page where the video goes, with Bilibili's
//! interface hidden and no ads (`Bilibili.js`, run before Bilibili.com's own scripts), and it plays, pauses and
//! seeks as the page asks, telling the page how it plays.
//!
//! It opens on Bilibili's home page, with no video: every video asked for is then Bilibili's own player asking
//! for it, and that request is written so Bilibili neither plays an ad nor holds the video back for one not
//! played (`Bilibili.js`). (A page opened on a video directly carries that video's data in the page, unasked.)
//! It shares the cabin page's WebView2 profile, so a sign-in there will be its too.
//!
//! It also asks Bilibili, as its own site does, what the Bilibili app shows: search results, search
//! suggestions, a video's Up next, and for a player signed in their Home and Subscriptions. The answers are
//! cut down there to a video's few fields, and here to only those, as plain text and images from Bilibili's
//! own image hosts ([`Answer`]).
//!
//! Signing in happens in Google's own page, in a window on the desktop (signin.rs), on this WebView's
//! profile. WestPlay learns nothing of the account: only whether Bilibili's page is signed in, and the picture
//! to show for it ([`Account`]), neither kept.

use std::cell::{Cell, RefCell};
use std::path::Path;

use crate::signin::{self, Confirming, Progress, SignInWindow};
use crate::speakers::{self, Equalizer, Head, Speaker};
use crate::webview::{self, OffscreenWebView, PageSize, Stage};

/// What the cabin page asks of Bilibili's player, or of Bilibili (`Bilibili` in ui/src/shared/host.ts).
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Command {
    /// Play a video (its 11-character id), from `start` seconds in, or (a series) its `part` (a series' own part,
    /// numbered from one: what the app's 选集 presses).
    Load {
        video: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part: Option<u32>,
    },
    Play,
    Pause,
    Seek {
        seconds: f64,
    },
    Volume {
        percent: u8,
    },
    /// Captions: this track (Bilibili's id for it: `.en`, `a.en` automatic鈥? on the video playing, its language
    /// on the videos after; `None`: none.
    Captions {
        track: Option<String>,
    },
    /// The picture's quality on every video: Bilibili's own choice (`auto`), or the best a video has at or under
    /// this one (`hd720`鈥?.
    Quality {
        quality: String,
    },
    /// The playback speed, on the video playing and the videos after (Bilibili's: 0.25 to 2).
    Rate {
        rate: f32,
    },
    /// The video's audio track: a dubbed one, or its original (its id, as the player lists it).
    AudioTrack {
        id: String,
    },
    /// Bilibili's Stable volume, on or off (the player's own preference: it keeps it).
    StableVolume {
        on: bool,
    },
    /// Bilibili's Voice boost, on or off (likewise).
    VoiceBoost {
        on: bool,
    },
    /// The truck's own speakers, for the sound to come from them (`None`: as it is, a truck whose speakers
    /// aren't known). The host's own (speakers.rs).
    Speakers {
        speakers: Option<Vec<Speaker>>,
    },
    /// The driver's head in the cab, the sound heard from there. The host's own (the telemetry).
    Head {
        #[serde(flatten)]
        head: Head,
    },
    /// The camera is outside the cab (`on`): the sound comes muffled, through its walls, and from as far
    /// off as the camera is from the cab (`distance`, metres; 0: not known). The host's own (camera.rs).
    Outside {
        on: bool,
        #[serde(default)]
        distance: f32,
    },
    /// The player's tone (the cab's Settings 鈥?Sound): bass, mid and treble, dB each. The host's own (settings).
    Equalizer {
        #[serde(flatten)]
        equalizer: Equalizer,
    },
    /// The music's spectrum wanted (the cab's Equalizer open) or not: the page says it as it plays. The host's
    /// own.
    Spectrum {
        on: bool,
    },
    /// Show the video at this rectangle of the frame (pixels), under the cabin page; `crop`: filling it,
    /// cropped (otherwise all of it, with bars where its shape isn't the rectangle's).
    Show {
        x: f32,
        y: f32,
        width: u32,
        height: u32,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        crop: bool,
    },
    /// The video fills its page, cropped, or shows all of it (`Show`'s `crop`, as Bilibili's page hears it).
    Crop {
        on: bool,
    },
    /// Show no video (it plays on, heard).
    Hide,
    /// Done with Bilibili: the WebView goes.
    Close,
    /// The language Bilibili answers in from now on (the cabin's, e.g. `tr`).
    Language {
        code: String,
    },
    /// Videos for a search, or the next ones of a search (`more`: the token its last answer gave).
    Search {
        id: u32,
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
        /// Channels only, or playlists only (Bilibili's own filters).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<SearchFilter>,
    },
    /// A playlist's page (its id): what it is and its videos, or its next videos (`more`).
    Playlist {
        id: u32,
        playlist: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
    },
    /// A channel's page (its id, `UC鈥): what it is and its videos, or its next videos (`more`).
    Channel {
        id: u32,
        channel: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
    },
    /// Bilibili's suggestions for what's typed so far.
    Suggest {
        id: u32,
        query: String,
    },
    /// A video's Up next.
    Related {
        id: u32,
        video: String,
    },
    /// A video's own page: its uploader, its numbers, its description, and the parts a series is played in.
    Detail {
        id: u32,
        video: String,
    },
    /// The signed-in player's Home or Subscriptions, or their next videos (`more`: the token the last
    /// answer gave).
    Feed {
        id: u32,
        which: Feed,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
    },
    /// The signed-in player's own page (the app's 我的): their picture, their level, their coins and their
    /// numbers.
    Profile {
        id: u32,
    },
    /// The signed-in player's 动态: the feed of the UP 主s they follow, or its next cards (`more`: the token
    /// its last answer gave).
    Dynamics {
        id: u32,
        /// Its videos only (`false`: everything they put up).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        video: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
    },
    /// The signed-in player's messages: the rows of `kind`'s tab, and how many of each kind are unread.
    Messages {
        id: u32,
        kind: Messages,
    },
    /// The signed-in player's own videos: their history, their watch later, or their favourites (`more`: the
    /// token its last answer gave).
    Library {
        id: u32,
        kind: Library,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
    },
    /// A video's comments (its 评论区 beside the player), the hottest or the newest, a page at a time (`more`:
    /// the page its last answer gave).
    Comments {
        id: u32,
        video: String,
        #[serde(default)]
        sort: Sort,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<String>,
    },
    /// A video's danmaku (the scrolling lines over its picture): every line, with when it comes.
    Danmaku {
        id: u32,
        video: String,
        /// A series' own part (`None`: the video's own).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part: Option<u32>,
    },
    /// Sends a line of danmaku to the video playing, at `at` seconds in (Bilibili's own box, `发送`).
    SendDanmaku {
        id: u32,
        video: String,
        text: String,
        at: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part: Option<u32>,
    },
    /// The signed-in player subscribes to a channel (`on`), or unsubscribes.
    Subscribe {
        id: u32,
        channel: String,
        on: bool,
    },
    /// Google's sign-in for Bilibili, in a window on the desktop (or that window to the front).
    SignIn,
    /// Signs Bilibili's page out.
    SignOut,
}

/// What a search finds, besides everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchFilter {
    Channels,
    Playlists,
}

/// A signed-in player's feed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Feed {
    /// The front page's recency feed.
    Home,
    /// Bilibili's ranking of everything (its hot page).
    Ranking,
    /// Its catalogue of series (bangumi).
    Bangumi,
    /// Its catalogue of films.
    Film,
    /// The videos of the UP 主s the player follows.
    Subscriptions,
    /// Bilibili's own selection for the player (its 精选: what it puts first of everything).
    Selected,
    /// Its live rooms (直播): each a room to watch in its own page.
    Live,
}

/// Which of the player's messages a page asks for (its 消息 page's tabs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Messages {
    /// The list of chats and notices (its own summary of what's new).
    Chat,
    /// Who replied to the player ("回复我的").
    Reply,
    /// Who called the player's name ("@我的").
    At,
    /// Who liked what the player wrote ("收到的赞").
    Like,
}

/// Which of the player's own videos a page asks for (its 我的 page's tabs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Library {
    /// What they watched ("历史记录").
    History,
    /// What they kept for later ("稍后再看").
    Later,
    /// What they favourited ("我的收藏").
    Favorites,
}

/// How a video's comments are ordered (its 评论区's own tabs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    /// The ones with the most likes first ("最热").
    #[default]
    Hot,
    /// The latest first ("最新").
    New,
}

impl Command {
    /// Whether it's a command Bilibili's page can take: a video id that is one, text of a sensible length.
    fn is_sound(&self) -> bool {
        let text = |text: &str| text.chars().count() <= 200;
        match self {
            Command::Load { video, .. } | Command::Related { video, .. } | Command::Detail { video, .. } => {
                is_video_id(video)
            }
            Command::Language { code } => crate::is_language_tag(code),
            Command::Search { query, more, .. } => text(query) && more.as_deref().is_none_or(is_token),
            Command::Feed { more, .. } => more.as_deref().is_none_or(is_token),
            Command::Dynamics { more, .. } | Command::Library { more, .. } => more.as_deref().is_none_or(is_token),
            Command::Comments { video, more, .. } => is_video_id(video) && more.as_deref().is_none_or(is_token),
            Command::Danmaku { video, .. } => is_video_id(video),
            Command::SendDanmaku { video, text, at, .. } => {
                is_video_id(video) && !text.trim().is_empty() && text.chars().count() <= 100 && at.is_finite() && *at >= 0.0
            }
            Command::Channel { channel, more, .. } => is_channel_id(channel) && more.as_deref().is_none_or(is_token),
            Command::Subscribe { channel, .. } => is_channel_id(channel),
            Command::Playlist { playlist, more, .. } => {
                is_playlist_id(playlist) && more.as_deref().is_none_or(is_token)
            }
            Command::Captions { track } => track.as_deref().is_none_or(is_track_id),
            Command::Quality { quality } => quality == "auto" || QUALITIES.contains(&quality.as_str()),
            Command::Speakers { speakers } => speakers.as_deref().is_none_or(speakers::are_sound),
            Command::Rate { rate } => rate.is_finite() && (0.25..=2.0).contains(rate),
            Command::AudioTrack { id } => is_audio_track_id(id),
            Command::Head { head } => head.is_sound(),
            Command::Outside { distance, .. } => distance.is_finite() && (0.0..10_000.0).contains(distance),
            Command::Equalizer { equalizer } => equalizer.clamped() == *equalizer,
            Command::Suggest { query, .. } => !query.trim().is_empty() && text(query),
            _ => true,
        }
    }
}

/// How the player plays, as the page hears it: only these fields, as read here (the Bilibili page's message
/// is Bilibili.com's page talking, so nothing else of it is passed on).
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct State {
    pub video: Option<String>,
    pub title: String,
    pub author: String,
    pub live: bool,
    /// Bilibili's player states: -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued.
    pub state: i32,
    pub time: f64,
    pub duration: f64,
    pub volume: f64,
    pub ad: bool,
    /// The video's captions, and the one shown (`None`: none).
    #[serde(default)]
    pub captions: Vec<CaptionTrack>,
    #[serde(default)]
    pub caption: Option<String>,
    /// The qualities the video has, best first; the one chosen (`auto`: Bilibili's own choice), and the one
    /// playing ("720p60").
    #[serde(default)]
    pub qualities: Vec<QualityLevel>,
    #[serde(default)]
    pub quality: String,
    #[serde(default, rename = "qualityLabel")]
    pub quality_label: String,
    /// The speeds the video can play at, and the one it plays at.
    #[serde(default)]
    pub rates: Vec<f32>,
    #[serde(default)]
    pub rate: f32,
    /// Its audio tracks (none: only its own), and the one playing.
    #[serde(default, rename = "audioTracks")]
    pub audio_tracks: Vec<AudioTrack>,
    #[serde(default, rename = "audioTrack")]
    pub audio_track: Option<String>,
    /// Bilibili's Stable volume and Voice boost, on or off (`None`: the video doesn't offer it).
    #[serde(default, rename = "stableVolume")]
    pub stable_volume: Option<bool>,
    #[serde(default, rename = "voiceBoost")]
    pub voice_boost: Option<bool>,
    /// How loud the truck's speakers play in each ear (left, right), 0 to 1, and how much the limiter takes
    /// off (dB, 0 or less) (`None`: the sound plays as it is).
    #[serde(default, rename = "cabLevel", skip_serializing_if = "Option::is_none")]
    pub cab_level: Option<[f32; 3]>,
    /// Before it plays, why it hasn't yet, for the log (`None` once it plays; never passed on to the cabin page).
    #[serde(default, skip_serializing)]
    pub diag: Option<Diagnosis>,
    /// While it plays, the picture not over the whole page, as the page reads it (its video elements and their
    /// boxes), for the log (`None`: it is; never passed on to the cabin page).
    #[serde(default, skip_serializing)]
    pub picture: Option<String>,
}

/// Why a video hasn't started, as the Bilibili page reads it (2026-10-27: players' videos stayed loading and the log
/// said nothing): Bilibili's playability status (`OK`, `UNPLAYABLE`, `LOGIN_REQUIRED`...) and its reason, the
/// player's last error code, the way its request was written (of how many) and how often Bilibili held the video
/// back, and the video element's ready and network states, its media error, the seconds buffered and the frames
/// decoded.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Diagnosis {
    pub playability: Option<String>,
    pub reason: Option<String>,
    pub error: Option<i32>,
    pub way: u32,
    pub ways: u32,
    pub holds: u32,
    pub ready: i32,
    pub network: i32,
    pub media_error: Option<i32>,
    pub buffered: f64,
    pub frames: u64,
}

impl Diagnosis {
    /// Its texts kept to a line of letters and the like (they're Bilibili.com's page's).
    fn sound(mut self) -> Self {
        let plain = |text: &mut Option<String>, most: usize| {
            if let Some(text) = text {
                text.retain(|c| !c.is_control());
                cut(text, most);
            }
        };
        plain(&mut self.playability, 32);
        plain(&mut self.reason, 80);
        if !self.buffered.is_finite() {
            self.buffered = 0.0;
        }
        self
    }
}

impl std::fmt::Display for Diagnosis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let or = |value: Option<i32>| value.map_or("none".to_owned(), |value| value.to_string());
        let held = if self.holds == 1 { "once".to_owned() } else { format!("{} times", self.holds) };
        write!(
            f,
            "playability {} ({}), player error {}, way {} of {}, held back {held}; ",
            self.playability.as_deref().unwrap_or("?"),
            self.reason.as_deref().unwrap_or("no reason"),
            or(self.error),
            self.way,
            self.ways,
        )?;
        write!(
            f,
            "video element ready {} network {}, media error {}, {:.1} s buffered, {} frames decoded",
            self.ready,
            self.network,
            or(self.media_error),
            self.buffered,
            self.frames
        )
    }
}

/// A video's captions: Bilibili's id for them and its name for them ("English (auto-generated)").
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CaptionTrack {
    pub track: String,
    pub name: String,
}

/// One of a video's audio tracks: the player's id for it, Bilibili's name ("English (US) original",
/// "Arabic"), and whether it's the original.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AudioTrack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub original: bool,
}

/// A quality a video has: Bilibili's name for it (`hd1080`) and its label ("1080p60").
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct QualityLevel {
    pub quality: String,
    #[serde(default)]
    pub label: String,
}

/// Bilibili's qualities, from the least.
const QUALITIES: [&str; 10] =
    ["tiny", "small", "medium", "large", "hd720", "hd1080", "hd1440", "hd2160", "hd2880", "highres"];

impl State {
    /// The state with only what can be shown: real track ids and qualities, texts cut.
    fn sound(mut self) -> Self {
        self.captions.retain(|track| is_track_id(&track.track));
        self.captions.truncate(100);
        for track in &mut self.captions {
            cut(&mut track.name, 80);
        }
        if self.caption.as_deref().is_some_and(|track| !is_track_id(track)) {
            self.caption = None;
        }
        self.qualities.retain(|level| QUALITIES.contains(&level.quality.as_str()));
        self.qualities.truncate(QUALITIES.len());
        for level in &mut self.qualities {
            cut(&mut level.label, 20);
        }
        if !QUALITIES.contains(&self.quality.as_str()) {
            self.quality = "auto".into();
        }
        cut(&mut self.quality_label, 20);
        self.rates.retain(|rate| rate.is_finite() && (0.0625..=16.0).contains(rate));
        self.rates.truncate(16);
        if !(self.rate.is_finite() && (0.0625..=16.0).contains(&self.rate)) {
            self.rate = 1.0;
        }
        self.audio_tracks.retain(|track| is_audio_track_id(&track.id));
        self.audio_tracks.truncate(64);
        for track in &mut self.audio_tracks {
            cut(&mut track.name, 80);
        }
        if self.audio_track.as_deref().is_some_and(|id| !is_audio_track_id(id)) {
            self.audio_track = None;
        }
        self.cab_level = self
            .cab_level
            .filter(|told| told.iter().all(|v| v.is_finite()))
            .map(|[left, right, limited]| [left.clamp(0.0, 1.0), right.clamp(0.0, 1.0), limited.clamp(-60.0, 0.0)]);
        self.diag = self.diag.map(Diagnosis::sound);
        if let Some(picture) = &mut self.picture {
            picture.retain(|c| !c.is_control());
            cut(picture, 300);
        }
        self
    }
}

/// `text` cut to `most` characters.
fn cut(text: &mut String, most: usize) {
    if let Some((at, _)) = text.char_indices().nth(most) {
        text.truncate(at);
    }
}

/// What Bilibili's page said (how the player plays, an answer, whether it's signed in), or whether the
/// sign-in window is open.
#[derive(Debug, PartialEq)]
pub enum Report {
    State(Box<State>),
    Answer(Box<Answer>),
    Account(Account),
    SignInWindow(bool),
    /// The sign-in window closed signed in, and Bilibili's page is yet to say it's signed in (`true`), or it
    /// has (or took too long: `false`).
    Confirming(bool),
    /// The music's spectrum, while the cab's Equalizer asks for it ([`speakers::read_spectrum`]).
    Spectrum(Vec<u8>),
}

/// Bilibili's answer to the page's question `answer` (its id): videos (and the token for more of them), or
/// suggestions; `failed` when Bilibili didn't answer. Only these fields pass (it's Bilibili.com's page
/// talking): see [`Answer::sound`].
#[derive(Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Answer {
    #[serde(rename(serialize = "id"))]
    pub answer: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub videos: Option<Vec<Video>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
    /// Channels found by a search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<Channel>>,
    /// The channel a channel's page is (its first answer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
    /// Playlists found by a search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlists: Option<Vec<Playlist>>,
    /// The playlist a playlist's page is (its first answer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist: Option<Playlist>,
    /// A video's own page: its player's data, uploader, numbers and parts (Bilibili's own, for its page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Detail>,
    /// The signed-in player's own page (the app's 我的).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<Profile>,
    /// The player's 动态: the cards of the feed of the UP 主s they follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<Vec<Dynamic>>,
    /// The player's messages: the rows of the tab asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,
    /// How many of the player's messages are unread, by kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread: Option<Unread>,
    /// A video's comments (its 评论区): the page of them asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<Vec<Comment>>,
    /// A video's danmaku (the lines that scroll over its picture).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danmaku: Option<Vec<Danmaku>>,
    /// Whether Bilibili took the line of danmaku the page sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
}

/// The signed-in player's own page: who they are, their level and their membership, their B coins and their
/// coins, and their numbers (how many videos they put up, who they follow, who follows them).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Profile {
    pub mid: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    /// Their level (0 to 6), as Bilibili's badge shows it.
    #[serde(default)]
    pub level: u32,
    /// Whether they're a 大会员.
    #[serde(default)]
    pub vip: bool,
    /// Their coins and their B coins, as Bilibili writes them ("880.6", "0.34").
    #[serde(default)]
    pub coins: String,
    #[serde(default)]
    pub bcoins: String,
    /// Their own words about themselves.
    #[serde(default)]
    pub sign: String,
    #[serde(default)]
    pub videos: u32,
    #[serde(default)]
    pub following: u32,
    #[serde(default)]
    pub followers: u32,
}

/// One card of the player's 动态: who put it up and when, what they did (Bilibili's own words: "投稿了视频"),
/// its words and its video or pictures, and its numbers.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Dynamic {
    /// Bilibili's own id for it (a card's key in the page).
    pub id: String,
    pub author: String,
    #[serde(default)]
    pub avatar: Option<String>,
    /// The author's id, to open their page (`None`: not known).
    #[serde(default, rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    /// "刚刚", "5分钟前"
    #[serde(default)]
    pub time: String,
    /// "投稿了视频", "发布了动态", "转发了动态"
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub text: String,
    /// Its video, when it's a video's (or a forwarded one's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<Video>,
    /// Its pictures, when it's a picture dynamic (Bilibili's own covers).
    #[serde(default)]
    pub pictures: Vec<String>,
    #[serde(default)]
    pub forwards: String,
    #[serde(default)]
    pub comments: String,
    #[serde(default)]
    pub likes: String,
}

/// One row of the player's messages: who wrote it, when, what it says, and whether it's still unread.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Message {
    /// Bilibili's own id for it (a row's key in the page).
    pub id: String,
    /// Which tab it's from: `reply`, `at`, `like`, or `notice` (a summary row of the chat list).
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub unread: bool,
}

/// How many of the player's messages are unread, by kind (its 消息 page's own numbers).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Unread {
    #[serde(default)]
    pub reply: u32,
    #[serde(default)]
    pub at: u32,
    #[serde(default)]
    pub like: u32,
    /// System notices (its own list, which has no per-person rows).
    #[serde(default)]
    pub notice: u32,
}

/// One comment of a video's 评论区: who wrote it, when, what it says, how many liked it, and how many replied
/// to it.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Comment {
    /// Bilibili's own id for it (a row's key in the page).
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    /// "3小时前"
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub text: String,
    /// Its likes ("8", "1.2万").
    #[serde(default)]
    pub likes: String,
    /// How many replies it has under it (its "共 N 条回复").
    #[serde(default)]
    pub replies: u32,
}

/// One line of a video's danmaku: when it comes (seconds into the video), what it says, and its colour.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Danmaku {
    /// Seconds into the video, at most its length.
    pub at: f64,
    pub text: String,
    /// Bilibili's own colour for it ("#FFFFFF"; none: white).
    #[serde(default)]
    pub color: Option<String>,
}

/// A video's own page, as its player and the page beside it show it: the uploader, the numbers, and the parts
/// a multi-part video is played in (each with its own `cid`).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Detail {
    pub video: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default, rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub pubdate: String,
    #[serde(default)]
    pub views: String,
    #[serde(default)]
    pub likes: String,
    #[serde(default)]
    pub coins: String,
    #[serde(default)]
    pub favorites: String,
    #[serde(default)]
    pub danmaku: String,
    /// Its parts, in order: a one-part video is a list of one.
    #[serde(default)]
    pub parts: Vec<Part>,
}

/// One part of a video (a series' own episode): its number, its `cid` (the page plays that), its title and
/// length.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Part {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub cid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub duration: String,
}

/// A channel as the Bilibili app shows it: its id, name and pictures, and Bilibili's own texts for it
/// ("@LofiGirl", "15.8M subscribers", "468 videos").
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Channel {
    pub channel: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub handle: String,
    #[serde(default)]
    pub subscribers: String,
    #[serde(default)]
    pub videos: String,
    /// Whether the signed-in player is subscribed to it (`None`: not signed in, or not known).
    #[serde(default)]
    pub subscribed: Option<bool>,
}

/// A playlist as the Bilibili app shows it: its id, title, channel and cover, and Bilibili's own texts for it
/// ("13 videos", "2M views"); in a search, how many of the search's videos come before it (`after`).
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Playlist {
    pub playlist: String,
    pub title: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default, rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub count: String,
    #[serde(default)]
    pub views: String,
    #[serde(default)]
    pub after: u32,
}

/// A video as the Bilibili app lists it. Its texts are Bilibili's, in the language asked for ("17K", "2 days
/// ago"); `duration` is empty for a live video.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Video {
    pub video: String,
    pub title: String,
    pub channel: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub views: String,
    #[serde(default)]
    pub age: String,
    #[serde(default)]
    pub live: bool,
    /// Its channel's id, to open the channel (`None`: not known).
    #[serde(default, rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    /// When it was watched or kept ("今天 15:19"), on the player's own lists (absent elsewhere).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub at: String,
}

impl Answer {
    /// The answer with only what can be shown: videos with a real id, texts cut to a sensible length,
    /// images only from Bilibili's image hosts, at most a page's worth.
    fn sound(mut self) -> Self {
        if let Some(videos) = &mut self.videos {
            videos.retain(|video| is_video_id(&video.video));
            // A playlist's page lists a hundred.
            videos.truncate(100);
            for video in videos {
                cut(&mut video.title, 200);
                cut(&mut video.channel, 100);
                for text in [&mut video.duration, &mut video.views, &mut video.age, &mut video.at] {
                    cut(text, 40);
                }
                for image in [&mut video.avatar, &mut video.thumbnail] {
                    if image.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                        *image = None;
                    }
                }
                if video.channel_id.as_deref().is_some_and(|id| !is_channel_id(id)) {
                    video.channel_id = None;
                }
            }
        }
        let channel_sound = |channel: &mut Channel| {
            cut(&mut channel.name, 100);
            for text in [&mut channel.handle, &mut channel.subscribers, &mut channel.videos] {
                cut(text, 60);
            }
            for image in [&mut channel.avatar, &mut channel.banner] {
                if image.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                    *image = None;
                }
            }
        };
        if let Some(channels) = &mut self.channels {
            channels.retain(|channel| is_channel_id(&channel.channel));
            channels.truncate(20);
            channels.iter_mut().for_each(channel_sound);
        }
        if self.channel.as_ref().is_some_and(|channel| !is_channel_id(&channel.channel)) {
            self.channel = None;
        }
        if let Some(channel) = &mut self.channel {
            channel_sound(channel);
        }
        let playlist_sound = |playlist: &mut Playlist| {
            cut(&mut playlist.title, 200);
            cut(&mut playlist.channel, 100);
            for text in [&mut playlist.count, &mut playlist.views] {
                cut(text, 40);
            }
            if playlist.thumbnail.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                playlist.thumbnail = None;
            }
            if playlist.channel_id.as_deref().is_some_and(|id| !is_channel_id(id)) {
                playlist.channel_id = None;
            }
        };
        if let Some(playlists) = &mut self.playlists {
            playlists.retain(|playlist| is_playlist_id(&playlist.playlist));
            playlists.truncate(20);
            playlists.iter_mut().for_each(playlist_sound);
        }
        if self.playlist.as_ref().is_some_and(|playlist| !is_playlist_id(&playlist.playlist)) {
            self.playlist = None;
        }
        if let Some(playlist) = &mut self.playlist {
            playlist_sound(playlist);
        }
        if let Some(suggestions) = &mut self.suggestions {
            suggestions.truncate(15);
            for suggestion in suggestions {
                cut(suggestion, 200);
            }
        }
        if let Some(detail) = &mut self.detail {
            cut(&mut detail.title, 200);
            cut(&mut detail.desc, 400);
            cut(&mut detail.channel, 100);
            for text in [&mut detail.pubdate, &mut detail.views, &mut detail.likes, &mut detail.coins, &mut detail.favorites, &mut detail.danmaku] {
                cut(text, 40);
            }
            if detail.avatar.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                detail.avatar = None;
            }
            if detail.channel_id.as_deref().is_some_and(|id| !is_channel_id(id)) {
                detail.channel_id = None;
            }
            // A series' parts: a hundred at most, each a real page Bilibili plays.
            detail.parts.retain(|part| part.cid > 0);
            detail.parts.truncate(100);
            for part in &mut detail.parts {
                cut(&mut part.title, 120);
                cut(&mut part.duration, 20);
            }
        }
        if self.profile.as_ref().is_some_and(|profile| !is_channel_id(&profile.mid)) {
            self.profile = None;
        }
        if let Some(profile) = &mut self.profile {
            cut(&mut profile.name, 40);
            cut(&mut profile.sign, 300);
            for text in [&mut profile.coins, &mut profile.bcoins] {
                cut(text, 20);
            }
            if profile.avatar.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                profile.avatar = None;
            }
        }
        if let Some(dynamics) = &mut self.dynamics {
            dynamics.retain(|dynamic| is_channel_id(&dynamic.id));
            dynamics.truncate(40);
            let video_sound = |video: &mut Video| {
                cut(&mut video.title, 200);
                cut(&mut video.channel, 100);
                for text in [&mut video.duration, &mut video.views, &mut video.age, &mut video.at] {
                    cut(text, 40);
                }
                for image in [&mut video.avatar, &mut video.thumbnail] {
                    if image.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                        *image = None;
                    }
                }
                if video.channel_id.as_deref().is_some_and(|id| !is_channel_id(id)) {
                    video.channel_id = None;
                }
            };
            for dynamic in dynamics {
                cut(&mut dynamic.author, 100);
                cut(&mut dynamic.time, 40);
                cut(&mut dynamic.action, 60);
                cut(&mut dynamic.text, 500);
                for text in [&mut dynamic.forwards, &mut dynamic.comments, &mut dynamic.likes] {
                    cut(text, 40);
                }
                if dynamic.avatar.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                    dynamic.avatar = None;
                }
                if dynamic.channel_id.as_deref().is_some_and(|id| !is_channel_id(id)) {
                    dynamic.channel_id = None;
                }
                if let Some(video) = &mut dynamic.video {
                    video_sound(video);
                    if !is_video_id(&video.video) {
                        dynamic.video = None;
                    }
                }
                // Only pictures Bilibili's own hosts have, three at most (a card's row).
                dynamic.pictures.retain(|url| is_bilibili_image(url));
                dynamic.pictures.truncate(3);
            }
        }
        if let Some(messages) = &mut self.messages {
            messages.retain(|message| is_channel_id(&message.id));
            messages.truncate(40);
            for message in messages {
                cut(&mut message.name, 100);
                for text in [&mut message.time, &mut message.text] {
                    cut(text, 200);
                }
                if message.avatar.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                    message.avatar = None;
                }
                if !["reply", "at", "like", "notice"].contains(&message.kind.as_str()) {
                    message.kind = String::from("notice");
                }
            }
        }
        if let Some(comments) = &mut self.comments {
            comments.retain(|comment| is_channel_id(&comment.id));
            comments.truncate(40);
            for comment in comments {
                cut(&mut comment.name, 100);
                cut(&mut comment.time, 40);
                cut(&mut comment.text, 500);
                cut(&mut comment.likes, 40);
                if comment.avatar.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
                    comment.avatar = None;
                }
            }
        }
        if let Some(danmaku) = &mut self.danmaku {
            // A busy video has thousands: the app draws the ones near the moment playing, so a few hundred of
            // the earliest do (its page's own list is longer than anything worth carrying over).
            danmaku.retain(|line| line.at.is_finite() && line.at >= 0.0 && !line.text.trim().is_empty());
            danmaku.sort_by(|a, b| a.at.total_cmp(&b.at));
            danmaku.truncate(4000);
            for line in danmaku {
                cut(&mut line.text, 100);
                if line.color.as_deref().is_some_and(|color| !is_color(color)) {
                    line.color = None;
                }
            }
        }
        if let Some(more) = &mut self.more {
            if !is_token(more) {
                self.more = None;
            }
        }
        self
    }
}

/// An image on Bilibili's own image hosts (covers, faces): its `hdslb.com` and `biliimg.com` CDNs, at any
/// subdomain.
pub(crate) fn is_bilibili_image(url: &str) -> bool {
    // `https://i0.hdslb.com/...`, `https://i2.hdslb.com/...`, `https://archive.biliimg.com/...`.
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else { return false };
    let host = rest.split('/').next().unwrap_or("");
    host == "hdslb.com"
        || host.ends_with(".hdslb.com")
        || host == "biliimg.com"
        || host.ends_with(".biliimg.com")
        || host == "bilibili.com"
        || host.ends_with(".bilibili.com")
}

/// A Bilibili channel id: `UC` and 22 characters of letters, digits, `-` and `_`.
fn is_channel_id(id: &str) -> bool {
    // An UP 涓?s mid: digits.
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
}

/// A Bilibili playlist id (`PL鈥, `OLAK5uy_鈥, `WL`鈥?: letters, digits, `-` and `_`, of a sensible length.
fn is_playlist_id(id: &str) -> bool {
    (2..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The player's id for an audio track (`251;Cg8KBWFjb250鈥: a format number and some base64).
fn is_audio_track_id(id: &str) -> bool {
    (1..=200).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b";-_=+/.".contains(&b))
}

/// Bilibili's id for a video's captions (`.en`, `a.en`, `.en.nP7-2PuUl7o`): letters, digits and `.-_`.
fn is_track_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
}

/// Bilibili's token for more of a list: letters, digits and a few signs, of a sensible length.
pub(crate) fn is_token(token: &str) -> bool {
    token.len() <= 4096 && token.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_=%".contains(&b))
}

/// A colour as a danmaku line takes it: `#RRGGBB`.
fn is_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// Whether Bilibili's page is signed in, and the picture to show for the player (from Google's image hosts).
/// Nothing else of the account: WestPlay neither asks nor keeps.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub signed_in: bool,
    #[serde(default)]
    pub avatar: Option<String>,
    /// The player's own name (Bilibili's own, for their page's header).
    #[serde(default)]
    pub name: String,
    /// Their `mid`: their own videos' page is that account's, in Bilibili's own interface.
    #[serde(default)]
    pub mid: Option<String>,
}

impl Account {
    fn sound(mut self) -> Self {
        if !self.signed_in || self.avatar.as_deref().is_some_and(|url| !is_bilibili_image(url)) {
            self.avatar = None;
        }
        if !self.signed_in {
            self.name.clear();
            self.mid = None;
        }
        cut(&mut self.name, 40);
        if self.mid.as_deref().is_some_and(|id| !is_channel_id(id)) {
            self.mid = None;
        }
        self
    }
}

/// What the page was told last of the cab: the truck (whose speakers it has), the head, whether the camera is
/// outside and how far, and the tone.
type Told = (Option<Option<Vec<Speaker>>>, Option<Head>, Option<(bool, f32)>, Option<Equalizer>, Option<bool>);

pub struct Bilibili {
    view: OffscreenWebView,
    /// The page's size, the picture kept within it.
    frame: PageSize,
    outbox: RefCell<Outbox>,
    /// The sign-in window while it's open.
    sign_in: RefCell<Option<SignInWindow>>,
    /// Once it closed signed in, until the page says it's signed in.
    confirming: Confirming,
    /// Reports of the host's own, for the next `take_reports`.
    pending: RefCell<Vec<Report>>,
    /// Whether its page last said it's signed in (`None`: it hasn't said yet this run). The
    /// reports above are taken each pass, so the answer is kept here as well, for the
    /// companion's accounts page to ask (`signed_in`).
    signed_in: Cell<Option<bool>>,
    /// The speakers the page has, and the head and camera it was told last ([`Bilibili::cab`]).
    cab: RefCell<Told>,
}

impl Bilibili {
    /// Opens Bilibili in a hidden WebView of `stage`'s frame (`width` x `height`), under the cabin page.
    pub fn start(stage: &Stage, width: u32, height: u32, user_data: &Path) -> Result<Self, webview::Error> {
        let view = OffscreenWebView::new(stage, width, height, user_data)?;
        view.log_as("bilibili")?;
        view.show(false)?;
        stage.add(&view, false)?;
        // The truck speakers' sound first: the page's script makes its own of it.
        view.add_startup_script(concat!(
            include_str!("cab_sound.js"),
            "\n",
            include_str!("bilibili.js")
        ))?;
        view.navigate("https://www.bilibili.com/")?;
        Ok(Self {
            view,
            frame: stage.page_size(),
            outbox: RefCell::new(Outbox::default()),
            sign_in: RefCell::new(None),
            confirming: Confirming::default(),
            pending: RefCell::new(Vec::new()),
            signed_in: Cell::new(None),
            cab: RefCell::new((None, None, None, None, None)),
        })
    }

    /// Carries out the page's command (`Close` is the caller's: [`Bilibili::close`]).
    pub fn command(&self, command: &Command) -> windows::core::Result<()> {
        match command {
            Command::Show { x, y, width, height, crop } => {
                self.pass(&Command::Crop { on: *crop })?;
                let page = self.frame.get();
                let (x, y, width, height) = within(page, *x, *y, *width, *height);
                // A small picture (Now playing's) is the whole page scaled down: Bilibili keeps playing at the
                // screen's quality, ready to be full screen again; a large one is the page at its size.
                if width * 2 < page.0 {
                    self.view.fit(page, x, y, width, height)?;
                } else {
                    self.view.place(x, y, width, height)?;
                }
                self.view.show(true)
            }
            Command::Hide => self.view.show(false),
            Command::Close => Ok(()),
            Command::SignIn => {
                let mut sign_in = self.sign_in.borrow_mut();
                if let Some(window) = sign_in.as_ref() {
                    window.bring_forward();
                } else {
                    match SignInWindow::open(self.view.environment(), &signin::BILIBILI) {
                        Ok(window) => {
                            *sign_in = Some(window);
                            self.pending.borrow_mut().push(Report::SignInWindow(true));
                        }
                        Err(e) => crate::startup::mark(&format!("host: the Bilibili sign-in window couldn't open: {e}")),
                    }
                }
                Ok(())
            }
            Command::SignOut => {
                self.view.delete_cookies()?;
                self.reload()
            }
            command if !command.is_sound() => Ok(()),
            _ => self.pass(command),
        }
    }

    /// The truck's speakers (none: its sound plays as it is), the driver's head and whether the camera is outside
    /// the cab, and how far (metres; 0: not known): the page is told the speakers when they change, the head when it
    /// moves, the camera when it goes in or out or a quarter metre further, the tone when it's set, the spectrum when it's wanted.
    pub fn cab(
        &self,
        speakers: Option<&[Speaker]>,
        head: Head,
        outside: bool,
        distance: f32,
        equalizer: Equalizer,
        spectrum: bool,
    ) -> windows::core::Result<()> {
        let mut cab = self.cab.borrow_mut();
        // The head first: the speakers' loudness is set from where it is as they're placed.
        if cab.1 != Some(head) && head.is_sound() {
            cab.1 = Some(head);
            self.pass_script(&Command::Head { head })?;
        }
        if cab.0.as_ref().map(Option::as_deref) != Some(speakers) {
            let speakers = speakers.map(<[_]>::to_vec);
            cab.0 = Some(speakers.clone());
            self.pass_script(&Command::Speakers { speakers })?;
        }
        let camera = (outside, if outside { (distance * 4.0).round() / 4.0 } else { 0.0 });
        if cab.2 != Some(camera) {
            cab.2 = Some(camera);
            self.pass_script(&Command::Outside { on: camera.0, distance: camera.1 })?;
        }
        if cab.3 != Some(equalizer) {
            cab.3 = Some(equalizer);
            self.pass_script(&Command::Equalizer { equalizer })?;
        }
        if cab.4 != Some(spectrum) {
            cab.4 = Some(spectrum);
            self.pass_script(&Command::Spectrum { on: spectrum })?;
        }
        Ok(())
    }

    /// Passes a command on to Bilibili's page **by running it there** rather than posting it.
    ///
    /// `PostWebMessageAsJson` reaches a document's `chrome.webview` listeners, and on this page it
    /// does not: the page runs Bilibili's own single-page app, which swaps the document's scripts
    /// as it navigates, and a listener the startup script added is gone with the document that had
    /// it — measured here with the page's own message counter at 0 while the graph it feeds stayed
    /// silent (owner 2026-11-25). `ExecuteScript` targets whatever document is current, so the call
    /// always lands on the graph the page has now.
    ///
    /// The command goes through the page's own dispatch (`run`), so the command's shape is the
    /// same as a posted one.
    fn pass_script(&self, command: &Command) -> windows::core::Result<()> {
        let message = serde_json::to_string(command).expect("commands serialise");
        // The page's `run` is the one `postMessage` would have called; the JSON is embedded as a
        // literal (`JSON.parse` of a quoted string, so nothing in it can end the script).
        let script = format!(
            "window.westplayRun && window.westplayRun({});",
            serde_json::to_string(&message).expect("a string is JSON")
        );
        self.view.run_script(&script, |_| ())
    }

    /// Passes a command on to Bilibili's page (held until it's there).
    fn pass(&self, command: &Command) -> windows::core::Result<()> {
        match self.outbox.borrow_mut().send(command) {
            Some(message) => self.view.post_json(&message),
            None => Ok(()),
        }
    }

    /// Bilibili's page again, from its home page (signed in or out since, here or in Bilibili Music, which shares
    /// the sign-in): it's owed the video and the rest again as it says it's there ([`Outbox::ready`]).
    pub fn reload(&self) -> windows::core::Result<()> {
        self.view.navigate("https://www.bilibili.com/")
    }

    /// Whether its page says it's signed in (`None`: it hasn't said yet this run). The
    /// companion's accounts page asks this, rather than remembering last run's answer.
    pub fn signed_in(&self) -> Option<bool> {
        self.signed_in.get()
    }

    /// What Bilibili's page said since the last call, oldest first, and whether the sign-in window closed.
    /// (Bilibili's page saying it's there gets what it's owed: [`Outbox::ready`]; signed in, it's loaded again.)
    pub fn take_reports(&self) -> Vec<Report> {
        let mut reports = std::mem::take(&mut *self.pending.borrow_mut());
    // What its page last said of the account, kept for the companion's accounts page to ask
    // (`signed_in`): the reports below are handed over and gone.
    for report in &reports {
        if let Report::Account(account) = report {
            self.signed_in.set(Some(account.signed_in));
        }
    }
        let progress = self.sign_in.borrow().as_ref().map(SignInWindow::progress);
        if let Some(progress @ (Progress::SignedIn | Progress::Closed)) = progress {
            self.sign_in.borrow_mut().take();
            reports.push(Report::SignInWindow(false));
            if progress == Progress::SignedIn {
                let _ = self.reload();
                self.confirming.start();
                reports.push(Report::Confirming(true));
            }
        }
        for message in self.view.take_page_messages() {
            if message == r#"{"ready":true}"# {
                for owed in self.outbox.borrow_mut().ready() {
                    let _ = self.view.post_json(&owed);
                }
            } else if let Some(bars) = speakers::read_spectrum(&message) {
                reports.push(Report::Spectrum(bars));
            } else if let Some(report) = read_report(&message) {
                reports.push(report);
            }
        }
        let signed_in = reports.iter().rev().find_map(|report| match report {
            Report::Account(account) => Some(account.signed_in),
            _ => None,
        });
        if self.confirming.ended(signed_in) {
            reports.push(Report::Confirming(false));
        }
        reports
    }

    /// Whether the WebView failed since the last call (it's then closed and started again when needed).
    pub fn failed(&self) -> bool {
        self.view.take_failure().is_some()
    }

    /// Takes the WebView off the frame; it closes as it's dropped.
    pub fn close(self, stage: &Stage) {
        let _ = stage.remove(&self.view);
    }
}

/// A message of Bilibili's page: an answer (it has `answer`), whether it's signed in (`account`), or how the
/// player plays.
fn read_report(message: &str) -> Option<Report> {
    let mut value: serde_json::Value = serde_json::from_str(message).ok()?;
    if let Some(account) = value.get_mut("account") {
        serde_json::from_value::<Account>(account.take()).ok().map(|account| Report::Account(account.sound()))
    } else if value.get("answer").is_some() {
        serde_json::from_value::<Answer>(value).ok().map(|answer| Report::Answer(Box::new(answer.sound())))
    } else {
        serde_json::from_value::<State>(value).ok().map(|state| Report::State(Box::new(state.sound())))
    }
}

/// The commands for Bilibili's page, as JSON. A message posted before the page is there goes nowhere, so
/// they're held until it says it is; and a page that loads again gets the video and its volume again
/// (Bilibili's home page reloads itself once on a new profile, `?themeRefresh=1`, and the video asked of the
/// page before goes with it).
#[derive(Default)]
struct Outbox {
    ready: bool,
    held: Vec<String>,
    video: Option<String>,
    volume: Option<String>,
    language: Option<String>,
    crop: Option<String>,
    captions: Option<String>,
    quality: Option<String>,
    rate: Option<String>,
    speakers: Option<String>,
    head: Option<String>,
    outside: Option<String>,
    equalizer: Option<String>,
    spectrum: Option<String>,
}

impl Outbox {
    /// `command` as the message to post now, or `None`: held until the page is there.
    fn send(&mut self, command: &Command) -> Option<String> {
        let message = serde_json::to_string(command).expect("commands serialise");
        match command {
            Command::Load { .. } => self.video = Some(message.clone()),
            Command::Volume { .. } => self.volume = Some(message.clone()),
            Command::Language { .. } => self.language = Some(message.clone()),
            Command::Crop { .. } => self.crop = Some(message.clone()),
            Command::Captions { .. } => self.captions = Some(message.clone()),
            Command::Quality { .. } => self.quality = Some(message.clone()),
            Command::Rate { .. } => self.rate = Some(message.clone()),
            Command::Speakers { .. } => self.speakers = Some(message.clone()),
            Command::Head { .. } => self.head = Some(message.clone()),
            Command::Outside { .. } => self.outside = Some(message.clone()),
            Command::Equalizer { .. } => self.equalizer = Some(message.clone()),
            Command::Spectrum { .. } => self.spectrum = Some(message.clone()),
            _ => {}
        }
        if self.ready {
            Some(message)
        } else {
            self.held.push(message);
            None
        }
    }

    /// The page says it's there: what to post to it, in order (the first time, what was held; after, the
    /// language, the cropping, the volume, the captions and quality, and the video again).
    fn ready(&mut self) -> Vec<String> {
        // 每次都说：`ready` 是页面（新文档）在说话，上一份文档听过的它一句都没听过。
        // 原来第一次只发 `held` 队列，而 `speakers` 早在 `cab()` 里被发送过就不在队列里了 ——
        // 页面于是没有扬声器可摆，视频只能平平地放（owner 2026-11-25）。`held` 里的还在它后面
        // 补上，队列中与缓存重复的不算问题：同一条命令说两遍没有副作用。
        let _ = std::mem::replace(&mut self.ready, true);
        // What was held first: those are the things the app asked of this page as it opened, in the
        // order it asked, and the cached state follows to fill in what the page has no word on.
        let mut out: Vec<String> = std::mem::take(&mut self.held);
        out.extend(
            [
            &self.language,
            &self.crop,
            &self.volume,
            &self.captions,
            &self.quality,
            &self.rate,
            &self.speakers,
            &self.head,
            &self.outside,
            &self.equalizer,
            &self.spectrum,
            &self.video,
        ]
        .into_iter()
        .flatten()
        .cloned(),
        );
        out
    }
}

/// A rectangle kept within the frame, at least a pixel each way.
pub(crate) fn within(
    (frame_width, frame_height): (u32, u32),
    x: f32,
    y: f32,
    width: u32,
    height: u32,
) -> (f32, f32, u32, u32) {
    let x = x.clamp(0.0, frame_width as f32 - 1.0);
    let y = y.clamp(0.0, frame_height as f32 - 1.0);
    let width = width.clamp(1, frame_width - x as u32);
    let height = height.clamp(1, frame_height - y as u32);
    (x, y, width, height)
}

/// A Bilibili video id: 11 characters of letters, digits, `-` and `_`.
pub(crate) fn is_video_id(id: &str) -> bool {
    // A BV number: `BV` and ten characters of Bilibili's own alphabet. A season (`ss` and its id) is a
    // series' own page, and `ep` an episode's: both open in Bilibili's, rather than a video's watch page. A
    // live room (`lv` and its id) is the same: its own page, where its stream plays.
    for prefix in ["ss", "ep", "lv"] {
        if let Some(rest) = id.strip_prefix(prefix) {
            return !rest.is_empty() && rest.len() <= 12 && rest.bytes().all(|b| b.is_ascii_digit());
        }
    }
    id.len() == 12 && id.starts_with("BV") && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `{"type":"youtube","site":"bilibili","state":{...}}`: how Bilibili's player plays (`null`: no Bilibili running).
///
/// `site` is what tells the two apart: YouTube's player reports the same shape (`youtube.rs`),
/// and the cabin page keeps one `state` per site — without it, a video Bilibili stopped playing
/// looked like YouTube's for the Dashboard's Now Playing (`owner 2026-11-25`).
pub fn state_message(state: Option<&State>) -> String {
    serde_json::json!({ "type": "youtube", "site": "bilibili", "state": state }).to_string()
}

/// `{"type":"youtubeAccount","signedIn":鈥?"avatar":鈥`: whether Bilibili's page is signed in.
pub fn account_message(account: &Account) -> String {
    let mut message = serde_json::to_value(account).expect("accounts serialise");
    message["type"] = "youtubeAccount".into();
    message.to_string()
}

/// `{"type":"youtubeSignIn","open":鈥`: whether the sign-in window is open.
pub fn sign_in_message(open: bool) -> String {
    serde_json::json!({ "type": "youtubeSignIn", "open": open }).to_string()
}

/// `{"type":"youtubeAnswer","id":鈥?"videos":[鈥,鈥`: Bilibili's answer to the page's question.
pub fn answer_message(answer: &Answer) -> String {
    let mut message = serde_json::to_value(answer).expect("answers serialise");
    message["type"] = "youtubeAnswer".into();
    message.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_pages_commands() {
        let read = |text: &str| serde_json::from_str::<Command>(text).ok();
        assert_eq!(
            read(r#"{"action":"load","video":"BV1GJ411x7h7"}"#),
            Some(Command::Load { video: "BV1GJ411x7h7".into(), start: None, part: None })
        );
        assert_eq!(read(r#"{"action":"seek","seconds":61.5}"#), Some(Command::Seek { seconds: 61.5 }));
        assert_eq!(
            read(r#"{"action":"show","x":0,"y":0,"width":1280,"height":720}"#),
            Some(Command::Show { x: 0.0, y: 0.0, width: 1280, height: 720, crop: false })
        );
        assert_eq!(
            read(r#"{"action":"show","x":8,"y":9,"width":372,"height":200,"crop":true}"#),
            Some(Command::Show { x: 8.0, y: 9.0, width: 372, height: 200, crop: true })
        );
        assert_eq!(read(r#"{"action":"close"}"#), Some(Command::Close));
        assert_eq!(read(r#"{"action":"explode"}"#), None);
        // As passed on to the Bilibili page.
        assert_eq!(
            serde_json::to_string(&Command::Volume { percent: 40 }).unwrap(),
            r#"{"action":"volume","percent":40}"#
        );
    }

    #[test]
    fn reads_the_players_own_pages() {
        let read = |text: &str| serde_json::from_str::<Command>(text).ok();
        assert_eq!(read(r#"{"action":"profile","id":7}"#), Some(Command::Profile { id: 7 }));
        assert_eq!(
            read(r#"{"action":"dynamics","id":8,"video":true,"more":"123"}"#),
            Some(Command::Dynamics { id: 8, video: true, more: Some("123".into()) })
        );
        assert_eq!(
            read(r#"{"action":"messages","id":9,"kind":"at"}"#),
            Some(Command::Messages { id: 9, kind: Messages::At })
        );
        assert_eq!(
            read(r#"{"action":"library","id":10,"kind":"favorites"}"#),
            Some(Command::Library { id: 10, kind: Library::Favorites, more: None })
        );
        assert_eq!(
            read(r#"{"action":"feed","id":11,"which":"selected"}"#),
            Some(Command::Feed { id: 11, which: Feed::Selected, more: None })
        );
        assert_eq!(
            read(r#"{"action":"feed","id":13,"which":"live"}"#),
            Some(Command::Feed { id: 13, which: Feed::Live, more: None })
        );
        // A series' part: the 选集's own number, played on the video's page (`?p=2`).
        assert_eq!(
            read(r#"{"action":"load","video":"BV1GJ411x7h7","part":2}"#),
            Some(Command::Load { video: "BV1GJ411x7h7".into(), start: None, part: Some(2) })
        );
        assert_eq!(
            read(r#"{"action":"danmaku","id":14,"video":"BV1GJ411x7h7"}"#),
            Some(Command::Danmaku { id: 14, video: "BV1GJ411x7h7".into(), part: None })
        );
        assert_eq!(
            read(r#"{"action":"sendDanmaku","id":15,"video":"BV1GJ411x7h7","text":"hi","at":12.5}"#),
            Some(Command::SendDanmaku {
                id: 15,
                video: "BV1GJ411x7h7".into(),
                text: "hi".into(),
                at: 12.5,
                part: None
            })
        );
        assert_eq!(
            read(r#"{"action":"comments","id":12,"video":"BV1GJ411x7h7","sort":"new","more":"2"}"#),
            Some(Command::Comments {
                id: 12,
                video: "BV1GJ411x7h7".into(),
                sort: Sort::New,
                more: Some("2".into())
            })
        );
        // The comments' order left out is the hottest.
        assert_eq!(
            read(r#"{"action":"comments","id":12,"video":"BV1GJ411x7h7"}"#),
            Some(Command::Comments { id: 12, video: "BV1GJ411x7h7".into(), sort: Sort::Hot, more: None })
        );
        assert!(serde_json::from_str::<Command>(r#"{"action":"messages","id":9,"kind":"x"}"#).is_err());
        // As passed on to the page: only what it asks.
        assert_eq!(
            serde_json::to_string(&Command::Dynamics { id: 1, video: false, more: None }).unwrap(),
            r#"{"action":"dynamics","id":1}"#
        );
        assert_eq!(
            serde_json::to_string(&Command::Library { id: 2, kind: Library::History, more: Some("5=6".into()) })
                .unwrap(),
            r#"{"action":"library","id":2,"kind":"history","more":"5=6"}"#
        );
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"library","id":2,"kind":"history","more":"5=6"}"#));
        assert!(!sound(r#"{"action":"library","id":2,"kind":"history","more":"5:6"}"#));
        assert!(sound(r#"{"action":"dynamics","id":3,"video":true,"more":"1234567890"}"#));
        assert!(sound(r#"{"action":"comments","id":4,"video":"BV1GJ411x7h7","more":"2"}"#));
        assert!(!sound(r#"{"action":"comments","id":4,"video":"not a video"}"#));
        assert!(sound(r#"{"action":"danmaku","id":5,"video":"BV1GJ411x7h7"}"#));
        assert!(!sound(r#"{"action":"danmaku","id":5,"video":"nope"}"#));
        assert!(sound(r#"{"action":"sendDanmaku","id":6,"video":"BV1GJ411x7h7","text":"a line","at":3}"#));
        // Not a line worth sending: none at all, too long, or a moment that isn't one.
        assert!(!sound(r#"{"action":"sendDanmaku","id":6,"video":"BV1GJ411x7h7","text":" ","at":3}"#));
        assert!(!sound(r#"{"action":"sendDanmaku","id":6,"video":"BV1GJ411x7h7","text":"a line","at":-1}"#));
    }

    #[test]
    fn keeps_only_sound_pages_of_the_players_own() {
        let answer: Answer = serde_json::from_str(
            r##"{"answer":1,"profile":{"mid":"2","name":"Name","avatar":"https://evil.example/track.png","level":6,
                "vip":true,"coins":"880.6","bcoins":"0.34","videos":43,"following":430,"followers":1439771},
                "dynamics":[
                  {"id":"42","author":"UP","avatar":"https://i0.hdslb.com/face","time":"刚刚","action":"投稿了视频",
                   "text":"words","video":{"video":"BV1GJ411x7h7","title":"a video","channel":"UP"},
                   "pictures":["https://evil.example/track.png","https://i1.hdslb.com/pic.jpg"],
                   "forwards":"1","comments":"2","likes":"3"},
                  {"id":"not-an-id","author":"UP"}],
                "messages":[
                  {"id":"7","kind":"reply","name":"Someone","avatar":"https://i2.hdslb.com/face","time":"2小时前",
                   "text":"hi","unread":true},
                  {"id":"8","kind":"weird","name":"x"}],
                "unread":{"reply":1,"at":0,"like":2,"notice":3},
                "comments":[
                  {"id":"9","name":"Reader","avatar":"https://i0.hdslb.com/face","time":"3小时前",
                   "text":"a comment","likes":"12","replies":2},
                  {"id":"bad id","name":"x"}],
                "danmaku":[
                  {"at":12.0,"text":"a line","color":"#FFFFFF"},
                  {"at":3.0,"text":"first"},
                  {"at":-1.0,"text":"no moment"},
                  {"at":9.0,"text":" ","color":"red"}]}"##,
        )
        .unwrap();
        let answer = answer.sound();
        let profile = answer.profile.unwrap();
        assert_eq!((profile.mid.as_str(), profile.level, profile.vip, profile.videos), ("2", 6, true, 43));
        // A picture that isn't Bilibili's is no picture at all.
        assert!(profile.avatar.is_none());
        // The card whose id isn't one is dropped, and its pictures: Bilibili's only.
        let dynamics = answer.dynamics.unwrap();
        assert_eq!(dynamics.len(), 1);
        assert_eq!(dynamics[0].pictures, ["https://i1.hdslb.com/pic.jpg"]);
        assert_eq!(dynamics[0].video.as_ref().map(|video| video.video.as_str()), Some("BV1GJ411x7h7"));
        let messages = answer.messages.unwrap();
        assert_eq!(messages.len(), 2);
        // A kind that isn't one is a notice.
        assert_eq!(messages[1].kind, "notice");
        assert_eq!(messages[0].kind, "reply");
        assert_eq!(answer.unread.map(|unread| (unread.reply, unread.notice)), Some((1, 3)));
        // The comment whose id isn't one is dropped; the other keeps what it says.
        let comments = answer.comments.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!((comments[0].name.as_str(), comments[0].likes.as_str(), comments[0].replies), ("Reader", "12", 2));
        assert_eq!(comments[0].avatar.as_deref(), Some("https://i0.hdslb.com/face"));
        // Danmaku: in time order, empty lines and moments that aren't ones dropped, a colour that isn't one
        // cleared.
        let danmaku = answer.danmaku.unwrap();
        assert_eq!(danmaku.iter().map(|line| line.at).collect::<Vec<_>>(), [3.0, 12.0]);
        assert_eq!(danmaku[0].text, "first");
        assert!(danmaku[0].color.is_none());
        assert_eq!(danmaku[1].color.as_deref(), Some("#FFFFFF"));
    }

    #[test]
    fn plays_only_what_looks_like_a_video() {
        assert!(is_video_id("BV1GJ411x7h7") && is_video_id("BV1xx411c7mD"));
        // A live room (直播), and a series' own pages, are pages of their own that play too.
        assert!(is_video_id("lv123456") && is_video_id("ss47836") && is_video_id("ep123456"));
        assert!(!is_video_id("lv") && !is_video_id("dQw4w9WgXc") && !is_video_id("BV1GJ411x7h7'") && !is_video_id("');alert(1)//"));
    }

    #[test]
    fn holds_commands_for_the_page_and_gives_a_reloaded_page_the_video_again() {
        let mut outbox = Outbox::default();
        let volume = Command::Volume { percent: 40 };
        let load = |video: &str| Command::Load { video: video.into(), start: None, part: None };
        // Before the page is there: held, in order.
        assert_eq!(outbox.send(&volume), None);
        assert_eq!(outbox.send(&load("BV1GJ411x7h7")), None);
        assert_eq!(outbox.send(&Command::Pause), None);
        // The page is there: it gets what was held, and — because it's a document of its own that
        // has heard none of this — the speakers and the head it needs for the sound to have a cab
        // to come from, not only the ones sent before it was there (`ready` is per document).
        let first = outbox.ready();
        assert_eq!(
            &first[..3],
            [
                r#"{"action":"volume","percent":40}"#,
                r#"{"action":"load","video":"BV1GJ411x7h7"}"#,
                r#"{"action":"pause"}"#
            ]
        );
        // (`speakers` and `head` are the cab's own, cached only once `cab()` has been told them; a
        // page that never had a cab gets none, which is what this Outbox alone has.)
        // There: posted at once.
        assert_eq!(outbox.send(&load("BV1GJ411x7h7")).as_deref(), Some(r#"{"action":"load","video":"BV1GJ411x7h7"}"#));
        assert_eq!(outbox.send(&Command::Play).as_deref(), Some(r#"{"action":"play"}"#));
        // The page loaded again: the volume and the latest video again, and the cab's speakers with
        // them (a new document hears none of it from the old one).
        let again = outbox.ready();
        assert!(again.iter().any(|line| line == r#"{"action":"volume","percent":40}"#));
        assert!(again.iter().any(|line| line == r#"{"action":"load","video":"BV1GJ411x7h7"}"#));
        // And the language, once there is one.
        assert!(outbox.send(&Command::Language { code: "tr".into() }).is_some());
        assert_eq!(outbox.ready()[0], r#"{"action":"language","code":"tr"}"#);
    }

    #[test]
    fn takes_only_sound_questions() {
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"search","id":1,"query":"ets2 scania"}"#));
        assert!(sound(r#"{"action":"search","id":2,"query":"","more":"EvgCEgtldHMy-_%3D"}"#));
        assert!(!sound(r#"{"action":"search","id":2,"query":"","more":"<script>"}"#));
        assert!(sound(r#"{"action":"suggest","id":3,"query":"ets2 sca"}"#));
        assert!(!sound(r#"{"action":"suggest","id":3,"query":"  "}"#));
        assert!(sound(r#"{"action":"related","id":4,"video":"BV1GJ411x7h7"}"#));
        assert!(!sound(r#"{"action":"related","id":4,"video":"x"}"#));
        assert!(sound(r#"{"action":"language","code":"tr"}"#));
        assert!(!sound(r#"{"action":"language","code":"tr&x=1"}"#));
        assert!(!sound(&format!(r#"{{"action":"search","id":1,"query":"{}"}}"#, "a".repeat(201))));
    }

    #[test]
    fn passes_on_only_a_videos_fields_and_youtubes_images() {
        let message = r#"{"answer":7,"videos":[
            {"video":"BV1GJ411x7h7","title":"ETS 2 1.60 SCANIA","channel":"BALPORSUGU","avatar":"https://i0.hdslb.com/a","thumbnail":"https://i2.hdslb.com/bfs/archive/cover.jpg","duration":"13:34","views":"17K","age":"2mo ago","live":false,"extra":"<img src=x>"},
            {"video":"BV1xx411c7mD","title":"lofi hip hop radio","channel":"Lofi Girl","avatar":"https://evil.example/a.png","thumbnail":"javascript:alert(1)","duration":"","views":"14M","age":"","live":true},
            {"video":"bad","title":"Not a video","channel":"x"}
        ],"more":"EvgC"}"#;
        let Some(Report::Answer(answer)) = read_report(message) else { panic!("an answer") };
        let videos = answer.videos.as_ref().unwrap();
        assert_eq!(videos.len(), 2);
        assert_eq!(videos[0].avatar.as_deref(), Some("https://i0.hdslb.com/a"));
        assert_eq!((videos[1].avatar.as_deref(), videos[1].thumbnail.as_deref()), (None, None));
        assert_eq!(
            answer_message(&answer),
            r#"{"id":7,"more":"EvgC","type":"youtubeAnswer","videos":[{"age":"2mo ago","avatar":"https://i0.hdslb.com/a","channel":"BALPORSUGU","duration":"13:34","live":false,"thumbnail":"https://i2.hdslb.com/bfs/archive/cover.jpg","title":"ETS 2 1.60 SCANIA","video":"BV1GJ411x7h7","views":"17K"},{"age":"","avatar":null,"channel":"Lofi Girl","duration":"","live":true,"thumbnail":null,"title":"lofi hip hop radio","video":"BV1xx411c7mD","views":"14M"}]}"#
        );
        let Some(Report::Answer(answer)) = read_report(r#"{"answer":8,"failed":true}"#) else { panic!() };
        assert_eq!(answer_message(&answer), r#"{"failed":true,"id":8,"type":"youtubeAnswer"}"#);
        // How the player plays is not an answer.
        let state = r#"{"video":"BV1GJ411x7h7","title":"t","author":"a","live":false,"state":1,"time":1,"duration":2,"volume":3,"ad":false}"#;
        assert!(matches!(read_report(state), Some(Report::State(_))));
    }

    #[test]
    fn passes_on_only_real_channels_and_their_youtube_pictures() {
        let message = r#"{"answer":9,"channels":[
            {"channel":"546195","name":"Lofi Girl","avatar":"https://i1.hdslb.com/a","banner":"https://evil.example/b.png","handle":"@LofiGirl","subscribers":"15.8M subscribers","videos":"468 videos","extra":"x"},
            {"channel":"not-a-channel","name":"Nope"}
        ],"channel":{"channel":"546195","name":"Lofi Girl"},
        "videos":[{"video":"BV1GJ411x7h7","title":"t","channel":"c","channelId":"javascript:x"}]}"#;
        let Some(Report::Answer(answer)) = read_report(message) else { panic!("an answer") };
        let channels = answer.channels.as_ref().unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].avatar.as_deref(), Some("https://i1.hdslb.com/a"));
        assert_eq!(channels[0].banner, None);
        assert_eq!(answer.channel.as_ref().map(|c| c.name.as_str()), Some("Lofi Girl"));
        assert_eq!(answer.videos.as_ref().unwrap()[0].channel_id, None);
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"channel","id":1,"channel":"546195"}"#));
        assert!(!sound(r#"{"action":"channel","id":1,"channel":"@LofiGirl"}"#));
        assert!(sound(r#"{"action":"search","id":2,"query":"ets2","filter":"channels"}"#));
        assert!(
            serde_json::from_str::<Command>(r#"{"action":"search","id":2,"query":"x","filter":"shorts"}"#).is_err()
        );
        assert!(sound(r#"{"action":"subscribe","id":3,"channel":"546195","on":true}"#));
        assert!(!sound(r#"{"action":"subscribe","id":3,"channel":"UC","on":true}"#));
        assert!(is_channel_id("546195") && !is_channel_id("UC0h3_FaL9Vdj1t4Bz3AGbl"));
    }

    #[test]
    fn passes_on_only_whether_signed_in_and_a_googles_picture() {
        let read = |text: &str| match read_report(text) {
            Some(Report::Account(account)) => account_message(&account),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            read(
                r#"{"account":{"signedIn":true,"avatar":"https://i0.hdslb.com/me=s88","name":"Someone","email":"x@y"}}"#
            ),
            r#"{"avatar":"https://i0.hdslb.com/me=s88","mid":null,"name":"Someone","signedIn":true,"type":"youtubeAccount"}"#
        );
        assert_eq!(
            read(r#"{"account":{"signedIn":true,"avatar":"https://evil.example/track.png"}}"#),
            r#"{"avatar":null,"mid":null,"name":"","signedIn":true,"type":"youtubeAccount"}"#
        );
        assert_eq!(
            read(r#"{"account":{"signedIn":false,"avatar":"https://i0.hdslb.com/me"}}"#),
            r#"{"avatar":null,"mid":null,"name":"","signedIn":false,"type":"youtubeAccount"}"#
        );
        assert_eq!(sign_in_message(true), r#"{"open":true,"type":"youtubeSignIn"}"#);
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"feed","id":1,"which":"subscriptions"}"#));
        assert!(!sound(r#"{"action":"feed","id":1,"which":"home","more":"no spaces"}"#));
        assert!(serde_json::from_str::<Command>(r#"{"action":"feed","id":1,"which":"history"}"#).is_err());
    }

    #[test]
    fn keeps_the_video_on_the_frame() {
        assert_eq!(within((1280, 720), 0.0, 0.0, 1280, 720), (0.0, 0.0, 1280, 720));
        assert_eq!(within((1280, 720), 900.0, 600.0, 1280, 720), (900.0, 600.0, 380, 120));
        assert_eq!(within((1280, 720), -50.0, 0.0, 0, 9999), (0.0, 0.0, 1, 720));
    }

    #[test]
    fn passes_on_only_the_players_own_fields() {
        let report = r#"{"video":"BV1GJ411x7h7","title":"Never Gonna Give You Up","author":"Rick Astley","live":false,
            "state":1,"time":12.3,"duration":213,"volume":100,"ad":false,"extra":"<img src=x>"}"#;
        let state: State = serde_json::from_str(report).unwrap();
        assert_eq!(
            state_message(Some(&state)),
            r#"{"site":"bilibili","state":{"ad":false,"audioTrack":null,"audioTracks":[],"author":"Rick Astley","caption":null,"captions":[],"duration":213.0,"live":false,"qualities":[],"quality":"","qualityLabel":"","rate":0.0,"rates":[],"stableVolume":null,"state":1,"time":12.3,"title":"Never Gonna Give You Up","video":"BV1GJ411x7h7","voiceBoost":null,"volume":100.0},"type":"youtube"}"#
        );
        assert_eq!(
            state_message(None),
            r#"{"site":"bilibili","state":null,"type":"youtube"}"#
        );
    }

    #[test]
    fn passes_on_only_real_captions_and_qualities() {
        let report = r#"{"video":"BV1GJ411x7h7","title":"t","author":"a","live":false,"state":1,"time":1,"duration":2,
            "volume":3,"ad":false,"captions":[{"track":".en","name":"English"},{"track":"a.en","name":"English (auto-generated)"},
            {"track":"<b>","name":"x"}],"caption":"a.en","qualities":[{"quality":"hd1080","label":"1080p60"},
            {"quality":"evil","label":"x"}],"quality":"hd9999","qualityLabel":"720p"}"#;
        let Some(Report::State(state)) = read_report(report) else { panic!("a state") };
        assert_eq!(state.captions.iter().map(|t| t.track.as_str()).collect::<Vec<_>>(), [".en", "a.en"]);
        assert_eq!(state.caption.as_deref(), Some("a.en"));
        assert_eq!(state.qualities, [QualityLevel { quality: "hd1080".into(), label: "1080p60".into() }]);
        assert_eq!((state.quality.as_str(), state.quality_label.as_str()), ("auto", "720p"));
        // An older page's report, without them.
        let older = r#"{"video":null,"title":"","author":"","live":false,"state":-1,"time":0,"duration":0,"volume":100,"ad":false}"#;
        let Some(Report::State(state)) = read_report(older) else { panic!("a state") };
        assert!(state.captions.is_empty() && state.caption.is_none() && state.quality == "auto");
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"captions","track":"a.en"}"#) && sound(r#"{"action":"captions","track":null}"#));
        assert!(!sound(r#"{"action":"captions","track":"en'); alert(1)"}"#));
        assert!(
            sound(r#"{"action":"quality","quality":"hd720"}"#) && sound(r#"{"action":"quality","quality":"auto"}"#)
        );
        assert!(!sound(r#"{"action":"quality","quality":"8k"}"#));
        // A reloaded page gets them again, before the video.
        let mut outbox = Outbox::default();
        outbox.ready();
        outbox.send(&Command::Quality { quality: "hd720".into() });
        outbox.send(&Command::Captions { track: Some(".en".into()) });
        outbox.send(&Command::Load { video: "BV1GJ411x7h7".into(), start: None, part: None });
        assert_eq!(
            outbox.ready(),
            [
                r#"{"action":"captions","track":".en"}"#,
                r#"{"action":"quality","quality":"hd720"}"#,
                r#"{"action":"load","video":"BV1GJ411x7h7"}"#
            ]
        );
    }

    #[test]
    fn tells_the_page_the_trucks_speakers_once_and_the_head_as_it_moves() {
        let head = Head { x: 0.0, y: 0.3, z: 0.1, heading: 0.0, pitch: 0.0 };
        let speakers = Command::Speakers { speakers: speakers::in_truck("scania.s_2016").map(<[_]>::to_vec) };
        let message = serde_json::to_string(&speakers).unwrap();
        assert!(
            message.starts_with(
                r#"{"action":"speakers","speakers":[{"band":[80.0,2000.0],"level":0.0,"channel":"left","at":[-1.065,"#
            ),
            "{message}"
        );
        assert_eq!(
            serde_json::to_string(&Command::Head { head }).unwrap(),
            r#"{"action":"head","x":0.0,"y":0.3,"z":0.1,"heading":0.0,"pitch":0.0}"#
        );
        // A truck whose speakers aren't known: the sound as it is.
        assert_eq!(
            serde_json::to_string(&Command::Speakers { speakers: None }).unwrap(),
            r#"{"action":"speakers","speakers":null}"#
        );
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(!sound(r#"{"action":"head","x":1e30,"y":0,"z":0,"heading":0,"pitch":0}"#));
        // A reloaded page gets them again, before the video.
        let mut outbox = Outbox::default();
        outbox.ready();
        outbox.send(&speakers);
        outbox.send(&Command::Head { head });
        outbox.send(&Command::Head { head: Head { x: 0.1, ..head } });
        outbox.send(&Command::Load { video: "BV1GJ411x7h7".into(), start: None, part: None });
        let again = outbox.ready();
        assert_eq!(again.len(), 3);
        assert!(again[0].contains("speakers") && again[1].contains(r#""x":0.1"#) && again[2].contains("load"));
    }

    #[test]
    fn passes_on_only_real_speeds_and_audio_tracks() {
        let report = r#"{"video":"0e3GPea1Tyg","title":"t","author":"a","live":false,"state":1,"time":1,"duration":2,
            "volume":3,"ad":false,"rates":[0.25,1,2,1e9],"rate":1.5,
            "audioTracks":[{"id":"251;Cg8KBWFjb250EgZkdWJiZWQKCgoEbGFuZxICYXI","name":"Arabic","original":false},
                {"id":"<script>","name":"x"}],"audioTrack":"251;Cg8KBWFjb250EgZkdWJiZWQKCgoEbGFuZxICYXI",
            "stableVolume":true,"voiceBoost":null}"#;
        let Some(Report::State(state)) = read_report(report) else { panic!("a state") };
        assert_eq!(state.rates, [0.25, 1.0, 2.0]);
        assert_eq!(
            (state.rate, state.audio_tracks.len(), state.stable_volume, state.voice_boost),
            (1.5, 1, Some(true), None)
        );
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"rate","rate":1.75}"#) && !sound(r#"{"action":"rate","rate":8}"#));
        assert!(sound(r#"{"action":"audioTrack","id":"251;Cg8KBWFjb250EgZkdWJiZWQKCgoEbGFuZxICYXI"}"#));
        assert!(!sound(r#"{"action":"audioTrack","id":"'); alert(1)"}"#));
        assert!(sound(r#"{"action":"stableVolume","on":false}"#) && sound(r#"{"action":"voiceBoost","on":true}"#));
    }

    #[test]
    fn passes_on_only_real_playlists() {
        let message = r#"{"answer":11,"playlists":[
            {"playlist":"PLrhhD0hhguUTrI2KtNh2pctWsQis_ZsNn","title":"ETS2 - Music","channel":"ProMods","channelId":"12345","thumbnail":"https://i2.hdslb.com/bfs/archive/cover2.jpg","count":"13 videos","after":2},
            {"playlist":"PL x","title":"Nope"},
            {"playlist":"PLOzDu-MXXLliO9fBNZOQTBDddoA3FzZUo","title":"lofi","thumbnail":"https://evil.example/a.png","channelId":"@x"}
        ],"playlist":{"playlist":"../VL","title":"x"}}"#;
        let Some(Report::Answer(answer)) = read_report(message) else { panic!("an answer") };
        let playlists = answer.playlists.as_ref().unwrap();
        assert_eq!(playlists.len(), 2);
        assert_eq!((playlists[0].after, playlists[0].count.as_str()), (2, "13 videos"));
        assert_eq!((playlists[1].thumbnail.as_deref(), playlists[1].channel_id.as_deref()), (None, None));
        assert_eq!(answer.playlist, None);
        let sound = |text: &str| serde_json::from_str::<Command>(text).unwrap().is_sound();
        assert!(sound(r#"{"action":"playlist","id":1,"playlist":"PLrhhD0hhguUTrI2KtNh2pctWsQis_ZsNn"}"#));
        assert!(sound(r#"{"action":"playlist","id":1,"playlist":"WL","more":"4qmFsgI"}"#));
        assert!(!sound(r#"{"action":"playlist","id":1,"playlist":"PL/../x"}"#));
        assert!(sound(r#"{"action":"search","id":2,"query":"ets2 music","filter":"playlists"}"#));
    }
}
