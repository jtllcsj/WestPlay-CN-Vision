import type { CustomWallpaper, IconSet } from './customize'

/**
 * Messages from the WestPlay host to the cabin page (WebView2 web messages). Mirrors the Rust side:
 * `westplay_shared::Telemetry`, `westplay_shared::TruckInfo`, the screen layout the game draws WestPlay on
 * (an id from crates/shared/screen_layouts.txt), and the app's dev commands.
 */

export interface Telemetry {
  x: number
  y: number
  z: number
  /** Turns (0..1), 0 = north. */
  heading: number
  pitch: number
  roll: number
  /** m/s, negative when reversing. */
  speed: number
  engineRpm: number
  gear: number
  displayedGear: number
  cruiseControlSpeed: number
  /** Litres. */
  fuel: number
  /** km. */
  fuelRange: number
  /** m / s / m/s (0 = no limit known). */
  navigationDistance: number
  navigationTime: number
  navigationSpeedLimit: number
  /** Minutes since day 0, 00:00 (day 0 is a Monday): the player's own time, which a job's times are in. */
  gameTime: number
  /** In a Convoy session, the session's time less the player's own, in-game minutes (0 out of one): the cab's
   * clock shows `gameTime` plus this ([`cabClock`]). */
  multiplayerTimeOffset: number
  localScale: number
  dashboardBacklight: number
  electricEnabled: boolean
  engineEnabled: boolean
  parkingBrake: boolean
  lowBeam: boolean
  highBeam: boolean
  leftBlinker: boolean
  rightBlinker: boolean
  paused: boolean

  adblue?: number
  /** The engine and its consumables, for the Vehicle app's overview: the brake air tank's pressure (psi,
   * `truck.brake.air.pressure`), the AdBlue left (litres, `truck.adblue`), the coolant's temperature (degrees
   * Celsius, `truck.water.temperature`) and the odometer (kilometres, `truck.odometer`). `engineKnown` says
   * which the game answered ([`ENGINE`]); an older host sends none, and then they read 0 for lack of an answer. */
  brakeAirPressure?: number
  cargoDamage?: number
  engineKnown?: number
  odometer?: number
  restStopKnown?: boolean
  /** In-game minutes until the game says the driver has to rest (its `rest.stop` channel), and whether the game
   * has said it: it says nothing while the game's fatigue simulation is off (§69). */
  restStopMinutes?: number
  /** The wheels' effective steering, -1 (right) to 1 (left), counterclockwise: the SDK's
   * `truck.effective.steering` (the reversing camera's guide lines swing by it). */
  steering: number
  /** Whether a trailer is hitched (the SDK's `trailer.connected`), how worn its chassis and wheels are and how
   * damaged its cargo is (0..1, as the rest; `wearKnown` again). */
  trailerConnected?: boolean
  trailerWearChassis?: number
  trailerWearWheels?: number
  waterTemperature?: number
  wearCab?: number
  wearChassis?: number
  /** How worn the truck's parts are, each 0 (as new) to 1: the SDK's `truck.wear.*` channels, the damage the
   * game shows in its own service and job screens. Which the game answered is `wearKnown` ([`WEAR`]); one it
   * doesn't have stays 0. An older host sends none: take them as unknown, not as no damage. */
  wearEngine?: number
  /** Bits naming the wear and damage channels the game answered: [`WEAR`]. */
  wearKnown?: number
  wearTransmission?: number
  wearWheels?: number
}

/** The time the cab's clock shows (minutes since day 0, 00:00): the game's, or in a Convoy session the
 * session's, shifted by `shift` minutes, the time zone where the truck is with the profile's time zones on
 * (the cab's `clockShift`, cabin/state/timeZone.ts: an hour on in Finland, an hour back in the UK).
 * Everything shown as a time goes by it; whether a job is late, by `gameTime`. */
export function cabClock(telemetry: Telemetry, shift: number): number {
  // A telemetry without it (none sent yet) is out of a session.
  return telemetry.gameTime + (telemetry.multiplayerTimeOffset || 0) + shift
}

export interface TruckInfo {
  /** e.g. "daf" */
  brandId: string
  /** Display brand, localized by the game, e.g. "DAF". */
  brand: string
  /** Model id, e.g. "vehicle.daf.2021". */
  id: string
  /** Display model name, localized by the game, e.g. "XG+". */
  name: string

  adblueCapacity?: number
  /** The truck's tanks in litres (`fuel.capacity`, `adblue.capacity`), given with the truck: absent, or 0, when
   * the game hasn't said — and then the Vehicle app shows how much is left without its share of the tank. */
  fuelCapacity?: number
}

/** The player's job (`westplay_shared::JobInfo`): names localized by the game. */
export interface JobInfo {
  /** There is a job. */
  active: boolean
  /** The cargo is on: delivering it (otherwise on the way to pick it up). */
  cargoLoaded: boolean
  /** A special transport (no companies). */
  special: boolean
  cargo: string
  /** kg. */
  cargoMass: number
  sourceCity: string
  sourceCompany: string
  destinationCity: string
  destinationCompany: string
  /** When the delivery window ends: minutes since day 0, 00:00 (as `Telemetry.gameTime`). */
  deliveryTime: number
  plannedDistanceKm: number
  income: number
}

/** What a keybind does: the music's volume a step up or down, or it paused or played on; or, a volume key held
 * (two seconds: owner 2026-11-03), the next song or the one before (or the song's start, past its first seconds). */
export type KeybindAction =
  'volumeUp' | 'volumeDown' | 'playPause' | 'nextTrack' | 'previousTrack' | 'brightnessDown' | 'brightnessUp'

export type HostMessage =
  | { type: 'telemetry'; data: Telemetry }
  | { type: 'guidance'; line: string }
  | { type: 'truck'; data: TruckInfo }
  | { type: 'job'; data: JobInfo }
  /** `null` until the game has drawn a WestPlay screen. Checked against the known ids on arrival. */
  | { type: 'screen'; layout: string | null }
  /** The settings the page applies: whether the truck's own strip stays beside WestPlay
   * (`westplay_host::settings::TruckStrip`), the wallpaper (`null`: none chosen yet) and the player's icon set
   * (`null`: WestPlay's own). Sent as the page loads and whenever they change. */
  | { type: 'trailer'; data: TrailerInfo }
  | {
      type: 'settings'
      truckStrip: string
      wallpaper?: string | null
      iconSet?: string | null
      /** Light or dark icons, or as the screen is (`IconStyle`). */
      iconStyle?: string
      /** Whether the map's buildings rise from it. */
      buildings?: boolean
      /** Whether its roads rise to their real heights. */
      roads?: boolean
      /** The player's own tone curve (Settings › Equalizer), and whether it plays (Custom) or not (Recommended). */
      equalizer?: Equalizer
      equalizerCustom?: boolean
      /** The language chosen in the companion (a game code, `de_de`); `null`: the game's. */
      language?: string | null
      /** How often WestPlay hands the game a new frame, chosen in Settings › Display & Performance (`'60'` or
       * `'30'`: `westplay_host::settings::RefreshRate`). Older hosts send none, meaning 60. */
      refreshRate?: string
      /** Whether the screen gives up some of its effects to run lighter, chosen in Settings › Display &
       * Performance (`westplay_host::settings::ScreenSettings::performance_mode`). An older host sends none,
       * meaning off. */
      performanceMode?: boolean
      /** The truck's own speakers, placed in its cab (Settings › Sound), or not: its sound plays as it is. */
      spatial?: boolean
    }
  /** The player's own wallpapers and icon sets (shared/customize.ts), their files at `CABIN_FILES`. */
  | { type: 'customize'; wallpapers: CustomWallpaper[]; iconSets: IconSet[] }
  /** The music's spectrum as it plays, while the page asks for it (`spectrum`): bars from the lows to the
   * highs, spaced by octaves, each 0 (silent) to 255; all 0 when it stops. */
  | { type: 'spectrum'; bars: number[] }
  /** Whether the host draws the screen's cursor (into the frames the game takes, where the mouse is at once), or
   * the page draws its own (crates/host cursor.rs). */
  | { type: 'cursor'; host: boolean }
  /** The cities whose garages the player owns, and their headquarters', from their latest save. */
  | { type: 'garages'; owned: string[]; headquarters: string | null }
  /** The game's answer to the page's `route` request `id`. */
  | { type: 'routeAnswer'; id: number; answer: RouteAnswer }
  /** WestPlay's own routes for the page's `routes` request `id` (none: no road graph, or no way there). */
  | { type: 'routes'; id: number; routes: PlannedRoute[] }
  /** What route `index` offered for the page's `routes` request `id` is drawn from, read from the game's files as the
   * game's route is (`route` and `routePaths`): its map nodes, `[x, z, forward x, forward z]`, the ways through its
   * junctions and how far right of its way it runs. Each as it's read: one across the map takes seconds. */
  | {
      type: 'offeredWays'
      id: number
      index: number
      nodes: [number, number, number, number][]
      paths: [number, [number, number][]][]
      shifts: [number, number][]
    }
  /** Whether the game's GPS follows the route picked (`follow` request `id`): `answer` "ownWay" when the
   * game takes another way however held, else the game's answer. */
  | { type: 'followed'; id: number; follows: boolean; answer: string }
  /** What stopping at each place of the page's `detours` request `id` adds to the trip (seconds on the map's
   * ground, in the order asked; `null`: not known). */
  | { type: 'detours'; id: number; seconds: (number | null)[] }
  /** What the player typed on their own keyboard, oldest first (a paste comes as its text). */
  | { type: 'typed'; events: TypedEvent[] }
  /** Whether the player's own keyboard types into the field selected. */
  | { type: 'typingState'; state: TypingState }
  /** Whether the player plays on a map by HVT, which WestPlay doesn't allow; and the game's language. */
  | { type: 'mapBlock'; hvt: boolean; language: string | null }
  /** Whether the player's profile has the game's time zones on (its clock the time where the truck is). */
  | { type: 'timeZones'; on: boolean }
  /** The player's profile's units (its `g_mph`): miles and mph (`true`), or kilometres and km/h; `null` when its
   * options don't say. */
  | { type: 'units'; mph: boolean | null }
  /** The game's GPS route: its map nodes in order, `[x, z, forward x, forward z, distance left, height]` (metres;
   * the road's direction, either way along it; the distance left from the node to the destination, as the
   * game counts distances). `null` without a route. */
  | {
      type: 'route'
      id: number | null
      nodes: [number, number, number, number, number, number][] | null
      /** The next coach stop's index in this route, when it continues past that stop. */
      activeEnd?: number | null
      /** Indices of the remaining intermediate coach stops, in route order. */
      coachStops?: readonly number[]
    }
  /** The ways through the route's junctions (the junction's own lanes, from the game's files): each from
   * the node it starts at to the next, as `[x, z]` points; and the turns there, by the same node: the
   * turn (`right`, `exit-left`, `roundabout`…), its angle (degrees, right positive) and a roundabout's
   * exit; and how far to the right of its way the route runs at its nodes on roads (metres: on the
   * carriageway it drives on, where a road is drawn as two), by node. */
  | {
      type: 'routePaths'
      id: number
      paths: [number, [number, number][]][]
      maneuvers: [number, string, number, number | null][]
      shifts: [number, number][]
    }
  /** The truck's progress along the route: the index of the map node ahead of it. */
  | { type: 'routeNext'; next: number }
  /** Open (or close) WestPlay on the truck's screen: the companion window's switch. */
  | { type: 'westplay'; open: boolean }
  /** The game's weather (`null` when it can't be read: no game, or a game version whose weather the host
   * doesn't know). */
  | { type: 'weather'; weather: GameWeather | null }
  /**
   * How the video player plays (`null`: it isn't running). `site` tells the two apart: YouTube's
   * player and Bilibili's are the same app over the same shell, and each reports this shape from
   * its own page (`youtube.rs` says `youtube`, `bilibili.rs` says `bilibili`). The cabin page keeps
   * one `state` per site — without it, a video one of them stopped looked like the other's on the
   * Dashboard's Now Playing.
   */
  | { type: 'youtube'; site?: 'youtube' | 'bilibili'; state: YouTubeState | null }
  /** YouTube's answer to the page's question `id` (a `youtube` command with an `id`). */
  | ({ type: 'youtubeAnswer'; id: number } & YouTubeAnswer)
  /** Whether the game's window shows (`false`: minimised, or on another desktop; in front or not: Lossless
   * Scaling's window is in front of it). */
  | { type: 'gameShown'; shown: boolean }
  /** The player's stops along the route `id`: their map nodes' indices in it, ascending (not WestPlay's own
   * waypoints, nor the destination). */
  | { type: 'routeStops'; id: number | null; stops: number[] }
  /** The game's menu gone while it's still paused: its still picture before the truck moves on. */
  | { type: 'menuGone'; gone: boolean }
  /** A keybind of the player's (the companion's Controls), pressed in the game. */
  | { type: 'keybind'; action: KeybindAction }
  /** The truck's speakers and the cab around them, for the cabin page's own player to place its sound the
   * same way the music pages do (`cab_sound.js`). NetEase Cloud Music's app draws itself here, so it gets
   * this message here rather than in a page of its own. */
  | {
      type: 'cab'
      speakers: Speaker[] | null
      head: CabHead
      outside: boolean
      distance: number
      equalizer: Equalizer
    }
  /** Whether the game runs. As it closes, the host closes the web players; the page powers off and forgets
   * its last telemetry. */
  | { type: 'game'; running: boolean }
  /** Whether the game runs under TruckersMP (the page shows its other players, from TruckersMP's map feed). */
  /** Whether the game runs under TruckersMP, and which game it is (`null`: not said yet). */
  | { type: 'truckersmp'; on: boolean; game?: 'ets2' | 'ats' | null }
  /** Under TruckersMP, the players around the player from its SDK (the plugin's): its game server's id, the
   * player's own number on it, and each other player's number, world x and z (m) and heading (turns); `null`s
   * and none when there are none to tell. */
  | { type: 'nearby'; server: number | null; local: number | null; players: [number, number, number, number][] }
  /** Whether YouTube is signed in, and the picture to show for the player: nothing else of the account. */
  | { type: 'youtubeAccount'; signedIn: boolean; avatar: string | null; name?: string; mid?: string | null }
  /** Whether Google's sign-in window is open on the PC's desktop. */
  | { type: 'youtubeSignIn'; open: boolean }
  /** The map the cab shows: its folder in WestPlay's maps (served at `https://westplay.maps/`), and when it
   * was built; `null` for none (none built yet). */
  | { type: 'map'; folder: string | null; built: number | null }
  /** Spotify's web player: whether it's signed in and playing (`null`: not running), and whether its sign-in
   * window is open on the PC's desktop. Nothing else of the account. */
  | { type: 'spotify'; signedIn: boolean | null; playing: boolean | null; signingIn: boolean }
  /** QQ Music's own player: signed in, playing, the player's avatar. */
  | { type: 'qqMusic'; signedIn: boolean | null; playing: boolean | null; avatar: string | null }

  /**
   * NetEase Cloud Music's local layer: which port it answers on, and whether an account is signed in.
   * The page asks the layer itself over the loopback (`apps/netease`), so this is only what it needs
   * to find it — the account's cookie never comes near the page. `port` is `null` while the layer
   * isn't running.
   */
  | { type: 'netease'; port: number | null; signedIn: boolean; nickname: string }  /** An answer to one of the app's questions (`ask`'s id). */
  | ({ type: 'qqMusicAnswer'; id: number; failed?: boolean } & Partial<QqMusicAnswers>)
  /** What QQ Music plays. */
  | { type: 'qqMusicPlayback'; playback: QqMusicPlayback | null }
  /** Spotify's answer to the Spotify app's question `id` (`failed`: Spotify didn't answer). */
  | ({ type: 'spotifyAnswer'; id: number; failed?: boolean } & Partial<SpotifyAnswers>)
  /** What Spotify plays (on WestPlay, or elsewhere on the account through Spotify Connect); `null`: nothing. */
  | { type: 'spotifyPlayback'; playback: SpotifyPlayback | null }
  /** Apple Music's web player: whether it's signed in and playing (`null`: not running), and whether its
   * sign-in window is open on the PC. */
  | {
      type: 'appleMusic'
      signedIn: boolean | null
      playing: boolean | null
      signingIn: boolean
      /** The account's picture (its Apple Music profile's), if it has one. */
      avatar: string | null
    }
  /** Apple Music's answer to the Music app's question `id` (`failed`: Apple Music didn't answer). */
  | ({ type: 'appleMusicAnswer'; id: number; failed?: boolean } & Partial<AppleMusicAnswers>)
  /** What Apple Music plays; `null`: nothing. */
  | { type: 'appleMusicPlayback'; playback: AppleMusicPlayback | null }
  /** The game's music folder (Player): where it is (`null`: the game's folder isn't known) and its songs. */
  | { type: 'musicFolder'; folder: string | null; songs: PlayerSong[] }
  /** What Player plays; `null`: nothing. */
  | { type: 'playerPlayback'; playback: PlayerPlayback | null }
  /** Radio's answer to a directory or schedule question `id` (`failed`: request failed). */
  | { type: 'radioAnswer'; id: number; failed?: boolean; error?: string; radio?: { kind: string; data: unknown } }
  /** What live radio plays; `null`: nothing. */
  | { type: 'radioPlayback'; playback: RadioPlayback | null }
  /** YouTube Music's web player: whether it's signed in and playing (`null`: not running), whether its sign-in
   * window is open on the PC, and the account's picture (Google's). YouTube's sign-in: they share it. */
  | {
      type: 'youtubeMusic'
      signedIn: boolean | null
      playing: boolean | null
      signingIn: boolean
      avatar: string | null
    }
  /** YouTube Music's answer to the app's question `id` (`failed`: YouTube Music didn't answer). */
  | ({ type: 'youtubeMusicAnswer'; id: number; failed?: boolean } & Partial<YouTubeMusicAnswers>)
  /** What YouTube Music plays; `null`: nothing. */
  | { type: 'youtubeMusicPlayback'; playback: YouTubeMusicPlayback | null }
  /** A Safari tab's page as it is now (`westplay_host::safari::TabState`). */
  | { type: 'safariTab'; tab: number; state: SafariTabState }
  /** A page opened a new window: a new tab `tab`, after the page's own (`opener`), its page already live. */
  | { type: 'safariOpened'; tab: number; opener: number }
  /** A page closed its own window (`window.close()`). */
  | { type: 'safariClosed'; tab: number }
  /** A tab's page let go (not live): loaded again from its address when shown. */
  | { type: 'safariLetGo'; tab: number }
  /** A page's dialog, waiting for the player's answer (an `answer` command). */
  | { type: 'safariDialog'; tab: number; dialog: SafariDialog }
  /** A tab's picture written, at `https://westplay.safari-tabs/tab-<tab>.png` (`version` new with each). */
  | { type: 'safariPicture'; tab: number; version: number }
  /** A page tried to download a file (a cab downloads nothing). */
  | { type: 'safariNoDownload'; tab: number }
  /** Whether any of Safari's pages plays sound. */
  | { type: 'safariPlaying'; playing: boolean }
  /** The hidden NetEase player's answer to a question its page's layer asked (
eteaseCommand's sk): the
   * question's own name carries the payload ({type:'neteaseAnswer', id, account:{…}}); ailed when it had
   * none. */
  | { type: 'neteaseAnswer'; id: number; failed?: boolean }
  /** The web app's page's own field: whether one has the focus, the text it holds, and which selection it is (a
   * number that changes each time one takes the focus). */
  | { type: 'webAppTyping'; on: boolean; text: string; selection: number }
  /** The web app's page: the site it shows and the page's own title (`null`: none running). */
  | { type: 'webApp'; state: WebAppState | null }
  /** What the web app's page plays, for the dashboard's Now Playing (`null`: nothing). */
  | { type: 'webAppMedia'; media: WebAppMedia | null }
  /** The words the web app's page has for what it plays, whole (`null`: none): the lyrics view's. */
  | { type: 'webAppLyrics'; lyrics: Lyrics | null }
  /** Where in the song the web app's page is (seconds): the lyrics view follows it. */
  | { type: 'webAppLyricAt'; at: number }
  /** The web app's page's own sound volume (0 to 1), for the lyrics view's slider. */
  | { type: 'webAppVolume'; volume: number }
  | { type: 'fatigue'; on: boolean }
  | { type: 'trafficLight'; light: TrafficLight | null }

/** The game's weather: the one there is and the one it's turning to (the same once it has); in game minutes,
 * how long until the game changes it next; and how wet the roads are, percent. Each of the last two `null`
 * (or missing, from an older host) when the host can't read it. */

export interface GameWeather {
  now: Sky
  coming: Sky
  changeIn?: number | null
  wetness?: number | null
}

/** One of the game's two weathers. */
export type Sky = 'clear' | 'rain'

/** Messages from the cabin page to the host. */
/** The players' tone controls, as a car stereo's: bass, mid and treble, in dB each (`EQUALIZER_MOST` either way;
 * the host's `speakers::Equalizer`). */
export interface Equalizer {
  bass: number
  mid: number
  treble: number
}

export const EQUALIZER_BANDS = ['bass', 'mid', 'treble'] as const satisfies readonly (keyof Equalizer)[]

/** How far each band goes, dB (the host's `Equalizer::MOST`). */
export const EQUALIZER_MOST = 6

export const FLAT: Equalizer = { bass: 0, mid: 0, treble: 0 }

/** The tone as the host may send it: each band a whole number of dB within the range (anything else flat). */
export function toEqualizer(value: unknown): Equalizer {
  const told = (value ?? {}) as Partial<Record<keyof Equalizer, unknown>>
  const band = (db: unknown) =>
    typeof db === 'number' && Number.isFinite(db)
      ? Math.max(-EQUALIZER_MOST, Math.min(EQUALIZER_MOST, Math.round(db)))
      : 0
  return { bass: band(told.bass), mid: band(told.mid), treble: band(told.treble) }
}

export type PageMessage =
  | { type: 'ready' }
  /** A startup stage reached (the boot screens shown), for the host's startup log. */
  | { type: 'startup'; stage: string }
  /** The cab's guidance (the next turn it shows and how far), every 30 s while there's a route
   * (cabin/maps/guidanceLog.ts). */
  | { type: 'guidance'; line: string }
  /** Whether WestPlay is open on the truck's screen (or the truck's original screen shows): posted as the
   * page connects and whenever it changes, for the companion window. */
  | { type: 'westplay'; open: boolean }
  /** The wallpaper chosen in Settings (wallpapers.ts), kept with WestPlay's settings (the companion's too). */
  | { type: 'wallpaper'; id: string }
  /** The Vehicle Bar (the truck's strip) kept or not, chosen in Settings › Appearance, kept the same way. */
  | { type: 'truckStrip'; strip: 'auto' | 'off' }
  /** The players' tone, set in Settings › Equalizer: the player's curve, and whether it plays (Custom) or the
   * speakers as tuned (Recommended); kept with WestPlay's settings and played by every player. */
  | ({ type: 'equalizer'; custom: boolean } & Equalizer)
  /** The music's spectrum wanted (Settings › Equalizer open) or not: the host answers `spectrum` as it plays. */
  | { type: 'spectrum'; on: boolean }
  /** The screen's brightness, set in Settings › Appearance: the slider's place, 0 to 1. The host scales the light of
   * the whole frame the game shows with it (crates/host brightness.rs). */
  | { type: 'brightness'; level: number }
  /** Whether what's under the cursor is light: the host draws it dark there. */
  | { type: 'cursorLight'; light: boolean }
  /** Where the host's cursor may be seen (page pixels): the CarPlay area, its corners round as its black ones, a
   * little inside the frame's edge (shell/cursorArea.ts). */
  | { type: 'cursorArea'; area: { left: number; top: number; right: number; bottom: number; radius: number } }
  /** A change to the game's GPS route (Maps' search), at a map node (its uid, hex) for going or adding a
   * stop; the host answers with `routeAnswer` and the same `id`. */
  | { type: 'route'; id: number; action: RouteAction; node?: string }
  /** The routes to a map node (its uid, hex; or to `also`, if a route comes to that last) from where the truck is, to
   * offer before Go (Directions). */
  | { type: 'routes'; id: number; node: string; also?: string }
  /** Go along the route at `index` of those offered for `routes` request `id`: the host hands it to the
   * game's GPS and answers `followed`. */
  | { type: 'follow'; id: number; index: number }
  /** What stopping at each of these map nodes (uids, hex) would add to the trip the game's route makes. */
  | { type: 'detours'; id: number; nodes: string[] }
  /** The text field selected, a number that changes with every selection (0: none), for the player's own
   * keyboard to type into while the truck stands still. */
  | { type: 'typing'; selection: number }
  /** A command for YouTube's player, or a question for YouTube (answered with `youtubeAnswer`). */
  | { type: 'youtube'; command: YouTubeCommand }
  /** A command for Bilibili's player, or a question for it (answered with `youtubeAnswer`: the same app draws
   * both, and Bilibili answers YouTube's shapes). */
  | { type: 'bilibili'; command: YouTubeCommand }
  /** A command for Spotify's web player (`westplay_host::spotify::Command`): `show` places it, in the frame's
   * pixels, where the Spotify app cuts the page open. */
  | { type: 'spotify'; command: SpotifyCommand }
  /** A command for Apple Music's web player (`westplay_host::apple_music::Command`). */
  | { type: 'appleMusic'; command: AppleMusicCommand }
  /** A command for YouTube Music's web player (`westplay_host::youtube_music::Command`). */
  | { type: 'youtubeMusic'; command: YouTubeMusicCommand }
  /** A command for Player's own player (`westplay_host::music_folder::Command`). */
  | { type: 'player'; command: PlayerCommand }
  /** A command for live radio (`westplay_host::radio::Command`). */
  | { type: 'radio'; command: RadioCommand }
  /** A command for QQ Music's own player. */
  | { type: 'qqMusic'; command: QqMusicCommand }

  /** A command for NetEase Cloud Music's hidden player (westplay_host::netease::Command): start it, sign
   * out, or a question its page's layer asks (sk), answered in 
eteaseAnswer with the same id. */
  | { type: 'neteaseCommand'; command: NeteaseCommand }

  /** The page asks for NetEase Cloud Music's layer: its port, or `null` when it isn't running. */
  | { type: 'neteasePort' }
  /** The page asks the layer to forget its account (the layer itself is told over the loopback). */
  | { type: 'neteaseSignOut' }  /** What the Safari app asks of its pages (`westplay_host::safari::Command`). */
  | { type: 'safari'; command: SafariCommand }
  | { type: 'refreshRate'; rate: '60' | '30' }
  | { type: 'trafficLights'; on: boolean }
  | { type: 'performanceMode'; on: boolean }
  | { type: 'spatial'; on: boolean }

  /** A Safari tab's page (`westplay_host::safari::TabState`): its address and title, whether it loads, whether it
   * can go back or forward, its icon's address, its colour at the top (a CSS colour), whether it's full screen,
   * the field pressed in it (a password's value as dots) and why it failed to load. */
  | { type: 'ime'; on: boolean }
  | { type: 'log'; text: string }
  | { type: 'webApp'; command: WebAppCommand }

export interface SafariTabState {
  url: string
  title: string
  loading: boolean
  back: boolean
  forward: boolean
  favicon?: string
  theme?: string
  /** The page's own ground (its background): what of the cab edges the page takes it. */
  ground?: string
  fullScreen: boolean
  /** The page has drawn something (its content loaded): the cab is cut open over it only then. */
  painted?: boolean
  /** `n`: which press of a field it is (a field pressed again is told again). */
  field?: { kind: string; value: string; n?: number }
  failed?: string
  /** A drop-down list pressed in the page: the cab shows its options (the browser's would be a window on the PC). */
  list?: SafariList
}

/** A page's drop-down list (`westplay_host::safari::List`): its options, the one picked (-1: none), and where it is
 * (frame pixels from the page's top left; none: not known). */
export interface SafariList {
  id: string
  options: { label: string; group?: string; disabled?: boolean }[]
  selected: number
  rect?: { x: number; y: number; width: number; height: number }
}

/** A page's dialog: `alert` (OK), `confirm` (OK or Cancel), `prompt` (text, with OK or Cancel). */
export interface SafariDialog {
  kind: 'alert' | 'confirm' | 'prompt'
  message: string
  text?: string
}

/** What the Safari app asks of its pages (`westplay_host::safari::Command`): a tab shown at a rectangle of the
 * frame (pixels; `covers`: where the cab draws over it there, its own and the mouse's), and the rest. */
export type SafariCommand =
  | {
      action: 'show'
      tab: number
      url: string
      x: number
      y: number
      width: number
      height: number
      zoom: number
      covers?: { x: number; y: number; width: number; height: number }[]
      /** The cab's appearance: the pages see it as the system's. */
      dark?: boolean
      /** Its bottom corners' rounding (frame pixels): the page cut to it. */
      radius?: number
      /** The cab's language (`tr-TR`): the pages are asked for in it. */
      language?: string
    }
  /** The cab's appearance changed: every page sees it (CSS `prefers-color-scheme`). */
  | { action: 'appearance'; dark: boolean }
  | { action: 'hide'; tab?: number }
  | { action: 'open'; tab: number; url: string }
  | { action: 'close'; tab: number }
  | { action: 'back'; tab: number }
  | { action: 'forward'; tab: number }
  | { action: 'reload'; tab: number }
  | { action: 'stop'; tab: number }
  | { action: 'type'; text: string }
  | { action: 'key'; key: 'backspace' | 'enter' }
  | { action: 'pause' }
  | { action: 'resume' }
  | { action: 'level'; level: number }
  | { action: 'answer'; tab: number; accept: boolean; text?: string }
  | { action: 'exitFullScreen'; tab: number }
  /** The option picked from the page's drop-down list (`index` left out: the list put away). */
  | { action: 'pick'; tab: number; list: string; index?: number }
  | { action: 'picture'; tab: number }
  /** The websites' data from `since` on (ms since 1970; 0: all of it). */
  | { action: 'forgetWebsites'; since?: number }

/** A song of the game's music folder (`westplay_host::music_folder::Song`): its file's name, and what its tags
 * say (none on a file without them): title, artist, length (seconds; 0: not known), cover (a file at
 * `https://westplay.music-covers/`). */
export interface PlayerSong {
  file: string
  title?: string
  artist?: string
  seconds: number
  cover?: string
}

export type PlayerRepeat = 'off' | 'all' | 'one'

/** What Player plays (`westplay_host::music_folder::Playback`): the song (its file), playing or not (loading:
 * asked to, its sound not come yet), where and how long (seconds), the options, the volume (0 to 1), and where
 * it is in the songs playing. */
export interface PlayerPlayback {
  file: string
  playing: boolean
  loading: boolean
  position: number
  duration: number
  shuffle: boolean
  repeat: PlayerRepeat
  volume: number | null
  index: number
  count: number
}

/** What Player asks of its player: the folder's songs (by file name, in its order) from the one at `index`
 * (shuffled: from any when there's no index), or play on (`play` alone); its controls. */
export type PlayerCommand =
  | { action: 'play'; files?: string[]; index?: number; shuffle?: boolean }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: PlayerRepeat }

/** What the Spotify app asks of Spotify's web player (`westplay_host::spotify::Command`), run hidden by the
 * host: its sign-in, a question (answered with `spotifyAnswer` and the same `id`), and the player's controls
 * (`play` with a context plays it, from `track`; without, it plays on). */
export type SpotifyCommand =
  /** Run (hidden), saying whether it's signed in (the Spotify app opened). */
  | { action: 'start' }
  | { action: 'signIn' }
  | { action: 'signUp' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: SpotifyQuestion }
  | { action: 'play'; context?: string; track?: string }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  /** The playing device's volume (Spotify's own), 0 to 100. */
  | { action: 'volume'; percent: number }
  /** The cab's fade, in the page's own player: its volume times `level` (0 to 1; 1, as Spotify has it). */
  | { action: 'fade'; level: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: SpotifyRepeat }
  /** Into Liked Songs or Your Library (`on`), or out of it. */
  | { action: 'save'; uri: string; on: boolean }

export type SpotifyRepeat = 'off' | 'all' | 'one'

/** What a row, a card or a shortcut shows: under its name, a card shows its description (or who it's by), a
 * row what it is and who it's by ("Playlist • Spotify", built in the cab's language). A song as a card (Home's
 * Trending songs) plays from its album (`context`). */
export interface SpotifyItem {
  uri: string
  kind: 'playlist' | 'album' | 'artist' | 'show' | 'liked' | 'track'
  name: string
  /** The playlist's owner, the album's artists, the podcast's publisher, the song's artists ("" for an
   * artist). */
  by: string
  description: string
  /** Liked Songs' count (its row says it in place of an owner). */
  songs?: number
  image: string | null
  /** The cover's own colour (Spotify extracts one for every image): pages and the player are washed in it. */
  color: string
  pinned?: boolean
  downloaded?: boolean
  /** In Your Library (Spotify says, signed in). */
  saved?: boolean
  /** A song's album. */
  context?: string
}

export interface SpotifyArtistName {
  uri: string
  name: string
}

export interface SpotifyTrack {
  uri: string
  name: string
  artists: SpotifyArtistName[]
  album: { uri: string; name: string }
  image: string | null
  color: string
  /** Seconds. */
  duration: number
  explicit: boolean
  /** In Liked Songs (Spotify says, signed in). */
  saved?: boolean
}

/** A playlist's or album's page. */
export interface SpotifyCollection {
  item: SpotifyItem
  description: string
  /** "Spotify", or the album's artist; and an album's year. */
  owner: string
  year: number | null
  tracks: SpotifyTrack[]
  /** Where its songs go on, while there are more than these: asked as the question's `more`. */
  more?: string
}

export interface SpotifyArtist {
  item: SpotifyItem
  listeners: number
  verified: boolean
  popular: SpotifyTrack[]
  releases: SpotifyItem[]
}

/** Home: Spotify's chips (each a facet of Home, its own words), its shortcuts, and its shelves. */
export interface SpotifyHome {
  chips: { id: string; label: string }[]
  shortcuts: SpotifyItem[]
  /** Its shelves; `recent`: Spotify's Recently played. */
  sections: { title: string; items: SpotifyItem[]; recent?: boolean }[]
}

export interface SpotifyCategory {
  name: string
  color: string
  /** A cover leaning in its corner, as Spotify's tiles have. */
  cover: SpotifyItem | null
}

export interface SpotifyResults {
  top: SpotifyItem | null
  /** The top result when it's a song. */
  topTrack: SpotifyTrack | null
  tracks: SpotifyTrack[]
  artists: SpotifyItem[]
  playlists: SpotifyItem[]
  albums: SpotifyItem[]
}

/** What the Spotify app asks Spotify: Home (a facet of it, `''` for all; the player's own signed in,
 * Spotify's public one otherwise), Your Library, Browse all, a search, a playlist's or album's page, an
 * artist's. */
export type SpotifyQuestion =
  | { ask: 'home'; facet: string; signedIn: boolean }
  | { ask: 'lyrics'; uri: string }
  | { ask: 'library' }
  | { ask: 'browse' }
  | { ask: 'search'; query: string }
  | { ask: 'collection'; uri: string; more?: string }
  | { ask: 'artist'; uri: string }

/** Each question's answer, by the question. */
export interface SpotifyAnswers {
  home: SpotifyHome
  library: SpotifyItem[]
  browse: SpotifyCategory[]
  search: SpotifyResults
  collection: SpotifyCollection
  artist: SpotifyArtist
  lyrics: Lyrics
}

/** What Spotify plays: the song, where it plays from, how far (seconds, as the host said it), the options,
 * and the songs to come. */
export interface SpotifyPlayback {
  track: SpotifyTrack | null
  context: { uri: string; name: string; kind: SpotifyItem['kind'] } | null
  playing: boolean
  position: number
  duration: number
  shuffle: boolean
  repeat: SpotifyRepeat
  /** The playing device's volume, 0 to 1 (`null`: none Spotify can set). */
  volume: number | null
  /** Playing on WestPlay's own device, not another of the account's (a phone); missing: taken as here. */
  here?: boolean
  next: SpotifyTrack[]
}

/** What the Music app asks of Apple Music's web player (`westplay_host::apple_music::Command`), run hidden by
 * the host: its sign-in (MusicKit's own window, on the PC), a question (answered with `appleMusicAnswer` and
 * the same `id`), and the player's controls. Ids are Apple Music's type and its own id: `albums:1440…`. */
export type AppleMusicCommand =
  /** Run (hidden), saying whether it's signed in (the Music app opened). */
  | { action: 'start' }
  | { action: 'signIn' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: AppleMusicQuestion }
  /** An album, playlist, station, artist or song (`id`), or a list of songs (`songs`), from `index`; with
   * neither, play on. */
  | {
      action: 'play'
      id?: string
      songs?: string[]
      index?: number
      shuffle?: boolean
      /** What `id` names: a playlist's number, or an album's or a singer's mid. */
      kind?: string
    }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: AppleMusicRepeat }
  /** A song of the queue, `offset` songs on. */
  | { action: 'jump'; offset: number }
  | { action: 'favorite'; id: string; on: boolean }
  | { action: 'add'; id: string }

export type AppleMusicRepeat = 'off' | 'all' | 'one'
export type AppleMusicShelf = 'recent' | 'playlists' | 'artists' | 'albums' | 'songs'

/** A card or row: what it is, its name, who it's by (an album's artist, a playlist's curator, a station's
 * tagline), Apple's editors' note, its cover (and a wide picture, Apple's editors' own) and the cover's
 * colour. */
export interface AppleMusicItem {
  id: string
  /** A folder: of the Library's playlists. */
  kind: 'album' | 'playlist' | 'artist' | 'station' | 'song' | 'curator' | 'folder' | 'top'
  name: string
  by: string
  note: string
  image: string | null
  wide: string | null
  color: string
  explicit: boolean
  /** A live station (Apple Music 1…). */
  live: boolean
  /** A big card's label ("UPDATED PLAYLIST"). */
  label?: string
  /** A favourite (the Library's lists say). */
  favorite?: boolean
}

export interface AppleMusicTrack {
  id: string
  name: string
  artist: string
  album: string
  albumId: string | null
  artistId: string | null
  image: string | null
  color: string
  /** Seconds. */
  duration: number
  explicit: boolean
  /** Its number on its album. */
  number: number | null
  /** A favourite (Apple Music says, signed in, for the song playing). */
  favorite?: boolean
}

export interface AppleMusicSection {
  title: string
  /** Big cards with the editors' pictures, a shelf of covers, what was played lately, or songs. */
  style: 'hero' | 'shelf' | 'recent' | 'songs'
  items: AppleMusicItem[]
  songs: AppleMusicTrack[]
}

/** Home, New, Radio or a category's page. */
export interface AppleMusicPage {
  sections: AppleMusicSection[]
}

export interface AppleMusicCollection {
  item: AppleMusicItem
  description: string
  genre: string
  year: number | null
  tracks: AppleMusicTrack[]

  /** Favoured playlists shown on the account's "Mine" page (QQ Music's own). */
  playlists: AppleMusicItem[]
}

export interface AppleMusicArtist {
  item: AppleMusicItem
  top: AppleMusicTrack[]
  sections: AppleMusicSection[]
}

export interface AppleMusicResults {
  top: AppleMusicItem[]
  songs: AppleMusicTrack[]
  artists: AppleMusicItem[]
  albums: AppleMusicItem[]
  playlists: AppleMusicItem[]
  stations: AppleMusicItem[]
}

export interface AppleMusicLibrary {
  items: AppleMusicItem[]
  songs: AppleMusicTrack[]
  /** The sorts the list has (Apple Music's own, named in the cab's language; a playlist's unnamed: the cab
   * names them), and the one it's in. */
  sorts: { id: string; label: string }[]
  sort: string | null
}

/** What the Music app asks Apple Music: Home (signed in), New, Radio, Search's categories, a category's page,
 * a search, an album's or playlist's page, an artist's, a list of the Library. */
export type AppleMusicQuestion =
  | { ask: 'home' }
  | { ask: 'new' }
  | { ask: 'radio' }
  | { ask: 'browse' }
  | { ask: 'curator'; id: string }
  | { ask: 'search'; query: string }
  | { ask: 'collection'; id: string }
  | { ask: 'artist'; id: string }
  | { ask: 'library'; shelf: AppleMusicShelf; sort?: string; folder?: string; language?: string }
  | { ask: 'lyrics'; id: string }

export interface AppleMusicAnswers {
  home: AppleMusicPage
  new: AppleMusicPage
  radio: AppleMusicPage
  browse: AppleMusicItem[]
  curator: AppleMusicPage
  search: AppleMusicResults
  collection: AppleMusicCollection
  artist: AppleMusicArtist
  library: AppleMusicLibrary
  lyrics: Lyrics
}

/** A song's words, as Apple Music and Spotify answer them (`westplay_host::lyrics::Lyrics`): line by line with
 * when each is sung (`synced`; seconds, `end` 0: until the next), where a stanza starts, where they're from
 * (Spotify's provider, Apple Music's songwriters), Spotify's colours for them, and whether they're written
 * right to left. No lines: no words for the song. */
export interface Lyrics {
  synced: boolean
  lines: { start: number; end: number; text: string; stanza?: boolean }[]
  provider?: string
  writers?: string[]
  colors?: { background: string; text: string; highlight: string }
  rtl?: boolean
}

/** What Apple Music plays: the song, where from (as the app asked for it), how far (seconds), whether it's
 * the song's 90-second preview (signed out, or no subscription), the options, the volume and what's next. */
export interface AppleMusicPlayback {
  track: AppleMusicTrack | null
  context: { id: string; name: string; kind: AppleMusicItem['kind'] } | null
  playing: boolean
  /** Playing as asked, its sound not come yet (loading, waiting for its stream, seeking): its time still. */
  loading?: boolean
  position: number
  duration: number
  preview: boolean
  shuffle: boolean
  repeat: AppleMusicRepeat
  /** The page's player's volume, 0 to 1. */
  volume: number | null
  next: AppleMusicTrack[]

  /** The player's own choice of it: a fade may lower `volume`, this one only the player sets (the
   * host keeps it for the next start). */
  chosen?: number
}

/** What the YouTube Music app asks of YouTube Music's web player (`westplay_host::youtube_music::Command`), run
 * hidden by the host: its sign-in (Google's, YouTube's own: they share it), a question (answered with
 * `youtubeMusicAnswer` and the same `id`), the language it answers in, and the player's controls (`play` with
 * `watch` plays it; without, it plays on). */
export type YouTubeMusicCommand =
  | { action: 'start' }
  | { action: 'signIn' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: YouTubeMusicQuestion }
  | { action: 'language'; code: string }
  | { action: 'play'; watch?: YouTubeMusicWatch }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: YouTubeMusicRepeat }
  /** The queue's song at `index` (its place in the whole queue). */
  | { action: 'jump'; index: number }
  | { action: 'like'; video: string; like: YouTubeMusicRating }
  /** A song into the queue: right after the one playing (`next`), or at its end. */
  | { action: 'queue'; video: string; next: boolean }
  /** An album or playlist (its playlist id) saved to the Library, or taken out. */
  | { action: 'save'; playlist: string; on: boolean }
  /** The Song/Video switch: the music video's picture (`true`), or the song. */
  | { action: 'video'; on: boolean }
  /** The music video's picture placed at this rectangle of the frame (pixels), under the page; or none. */
  | { action: 'show'; x: number; y: number; width: number; height: number }
  | { action: 'hide' }

export type YouTubeMusicRepeat = 'off' | 'all' | 'one'
export type YouTubeMusicRating = 'like' | 'dislike' | 'none'
export type YouTubeMusicShelf = 'playlists' | 'songs' | 'albums' | 'artists' | 'subscriptions' | 'history'
export type YouTubeMusicKind =
  'song' | 'video' | 'episode' | 'album' | 'playlist' | 'artist' | 'channel' | 'podcast' | 'mood'

/** Playing something: a song (`video`), from a list (`playlist`, at `index`), with YouTube's own `params` (a
 * radio, a shuffle), as YouTube Music's answers give them. */
export interface YouTubeMusicWatch {
  video?: string
  playlist?: string
  params?: string
  index?: number
}

/** A page of YouTube Music's: its id, and its params (a mood's, a chart's). */
export interface YouTubeMusicBrowse {
  id: string
  params?: string
}

/** A card or a row: what it is, its title and YouTube Music's own line under it, its picture, its page,
 * playing it, a song's id, artist and album pages and length, a mood's colour, its place (a number). */
export interface YouTubeMusicItem {
  kind: YouTubeMusicKind
  title: string
  subtitle: string
  image?: string
  browse?: YouTubeMusicBrowse
  play?: YouTubeMusicWatch
  video?: string
  artist?: YouTubeMusicBrowse
  album?: YouTubeMusicBrowse
  duration?: string
  explicit?: boolean
  color?: string
  rank?: string
}

/** Square covers side by side; songs in columns of four (Quick picks); songs one under another; mood
 * buttons, all of them or in rows side by side; words. */
export type YouTubeMusicStyle = 'shelf' | 'columns' | 'songs' | 'moods' | 'moodRows' | 'about'

export interface YouTubeMusicSection {
  title: string
  strapline?: string
  style: YouTubeMusicStyle
  items: YouTubeMusicItem[]
  text?: string
  more?: YouTubeMusicBrowse
}

export interface YouTubeMusicChip {
  label: string
  params?: string
  selected?: boolean
}

/** Home, Explore, a page, a song's Related, the Library's: its choices, shelves and the token for more. */
export interface YouTubeMusicPage {
  title?: string
  /** The picture YouTube Music shows behind its top (Home's, as its mood is). */
  background?: string
  chips: YouTubeMusicChip[]
  sections: YouTubeMusicSection[]
  more?: string
}

export interface YouTubeMusicCollection {
  kind: 'album' | 'playlist'
  title: string
  subtitle: string
  second: string
  by: string
  byPage?: YouTubeMusicBrowse
  image?: string
  description: string
  play?: YouTubeMusicWatch
  shuffle?: YouTubeMusicWatch
  radio?: YouTubeMusicWatch
  playlist?: string
  saved?: boolean
  tracks: YouTubeMusicItem[]
  sections: YouTubeMusicSection[]
  more?: string
}

export interface YouTubeMusicArtist {
  name: string
  description: string
  image?: string
  subscribers: string
  shuffle?: YouTubeMusicWatch
  radio?: YouTubeMusicWatch
  sections: YouTubeMusicSection[]
}

export interface YouTubeMusicResults {
  top?: YouTubeMusicItem
  topSongs: YouTubeMusicItem[]
  chips: YouTubeMusicChip[]
  items: YouTubeMusicItem[]
  more?: string
}

export interface YouTubeMusicSuggestions {
  texts: string[]
  items: YouTubeMusicItem[]
}

export interface YouTubeMusicLyrics {
  text: string
  source: string
}

export type YouTubeMusicQuestion =
  | { ask: 'home'; chip?: string; more?: string }
  | { ask: 'explore' }
  | { ask: 'browse'; page: YouTubeMusicBrowse }
  | { ask: 'search'; query: string; filter?: string; more?: string }
  | { ask: 'suggest'; query: string }
  | { ask: 'collection'; id: string; more?: string }
  | { ask: 'artist'; id: string }
  | { ask: 'library'; shelf: YouTubeMusicShelf }
  | { ask: 'lyrics'; video: string }
  | { ask: 'related'; video: string }

export interface YouTubeMusicAnswers {
  home: YouTubeMusicPage
  explore: YouTubeMusicPage
  browse: YouTubeMusicPage
  search: YouTubeMusicResults
  suggest: YouTubeMusicSuggestions
  collection: YouTubeMusicCollection
  artist: YouTubeMusicArtist
  library: YouTubeMusicPage
  lyrics: YouTubeMusicLyrics
  related: YouTubeMusicPage
}

/** The song playing: its id, title, artists (and their page), album (and its page), cover, length, kind, like,
 * and whether it has a music video to show. */
export interface YouTubeMusicTrack {
  video: string
  title: string
  artists: string
  artist?: YouTubeMusicBrowse
  album: string
  albumPage?: YouTubeMusicBrowse
  image?: string
  duration: number
  kind: YouTubeMusicKind
  like: YouTubeMusicRating
  hasVideo: boolean
}

/** What YouTube Music plays: the song, playing (and waiting for its sound), how far, the options, the volume,
 * the Song/Video switch, the queue around the song (`queue` from the whole queue's `start`; `index`: the
 * song's place in the whole queue), what it plays from, and whether an ad plays. */
export interface YouTubeMusicPlayback {
  track: YouTubeMusicTrack
  playing: boolean
  loading: boolean
  position: number
  duration: number
  shuffle: boolean
  repeat: YouTubeMusicRepeat
  volume: number | null
  video: boolean
  queue: YouTubeMusicItem[]
  start: number
  index: number
  from: string
  ad: boolean
}

/** What the YouTube app asks of YouTube's player (hidden, under the page) or of YouTube (`westplay_host::
 * youtube::Command`). `show` places the video, in the frame's pixels. */
export type YouTubeCommand =
  | { action: 'load'; video: string; start?: number; part?: number }
  | { action: 'play' }
  | { action: 'pause' }
  | { action: 'seek'; seconds: number }
  | { action: 'volume'; percent: number }
  | { action: 'captions'; track: string | null }
  | { action: 'quality'; quality: string }
  | { action: 'rate'; rate: number }
  | { action: 'detail'; id: number; video: string }
  | { action: 'audioTrack'; id: string }
  | { action: 'stableVolume'; on: boolean }
  | { action: 'voiceBoost'; on: boolean }
  | { action: 'show'; x: number; y: number; width: number; height: number; crop?: boolean }
  | { action: 'hide' }
  | { action: 'close' }
  | { action: 'language'; code: string }
  | { action: 'search'; id: number; query: string; more?: string; filter?: 'channels' | 'playlists' }
  | { action: 'playlist'; id: number; playlist: string; more?: string }
  | { action: 'channel'; id: number; channel: string; more?: string }
  | { action: 'subscribe'; id: number; channel: string; on: boolean }
  | { action: 'suggest'; id: number; query: string }
  | { action: 'related'; id: number; video: string }
  | { action: 'feed'; id: number; which: 'home' | 'selected' | 'ranking' | 'bangumi' | 'film' | 'subscriptions' | 'live'; more?: string }
  | { action: 'profile'; id: number }
  | { action: 'dynamics'; id: number; video?: boolean; more?: string }
  | { action: 'messages'; id: number; kind: 'chat' | 'reply' | 'at' | 'like' }
  | { action: 'library'; id: number; kind: 'history' | 'later' | 'favorites'; more?: string }
  | { action: 'comments'; id: number; video: string; sort?: 'hot' | 'new'; more?: string }
  | { action: 'danmaku'; id: number; video: string; part?: number }
  | { action: 'sendDanmaku'; id: number; video: string; text: string; at: number; part?: number }
  | { action: 'signIn' }
  | { action: 'signOut' }

/** How YouTube's player plays. `state`: -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued. */
export interface YouTubeState {
  video: string | null
  title: string
  author: string
  live: boolean
  state: number
  /** Seconds. */
  time: number
  duration: number
  volume: number
  ad: boolean
  /** The video's captions (YouTube's id for each: `.en`, `a.en` automatic) and the one shown (`null`: none). */
  captions: { track: string; name: string }[]
  caption: string | null
  /** The qualities the video has, best first (`hd1080`, "1080p60"); the one chosen (`auto`: YouTube's own
   * choice), and the one playing ("720p"). */
  qualities: { quality: string; label: string }[]
  quality: string
  qualityLabel: string
  /** The speeds it can play at and the one it plays at; its audio tracks (none: only its own) and the one
   * playing; YouTube's Stable volume and Voice boost (`null`: the video doesn't offer it). */
  rates: number[]
  rate: number
  audioTracks: { id: string; name: string; original: boolean }[]
  audioTrack: string | null
  stableVolume: boolean | null
  voiceBoost: boolean | null
}

/** A video as the YouTube app lists it; its texts are YouTube's own, in the cabin's language. */
export interface YouTubeVideo {
  video: string
  title: string
  channel: string
  avatar: string | null
  thumbnail: string | null
  /** "13:34"; empty for a live video. */
  duration: string
  /** "17K", "14 Mn" */
  views: string
  /** "2 days ago" */
  age: string
  live: boolean
  /** Its channel's id, to open the channel (absent: not known). */
  channelId?: string
  /** When it was watched or kept ("今天 15:19"), on the player's own lists. */
  at?: string
}

/** A channel; its texts are YouTube's ("@LofiGirl", "15.8M subscribers", "468 videos"). */
export interface YouTubeChannel {
  channel: string
  name: string
  avatar: string | null
  banner: string | null
  handle: string
  subscribers: string
  videos: string
  /** Whether the signed-in player is subscribed (`null`: not signed in, or not known). */
  subscribed?: boolean | null
}

/** A playlist; its texts are YouTube's ("13 videos", "2M views"). In a search, `after`: how many of the
 * search's videos come before it. */
export interface YouTubePlaylist {
  playlist: string
  title: string
  channel: string
  channelId?: string
  thumbnail: string | null
  count: string
  views: string
  after: number
}

/** YouTube's answer: videos (and the token for more), suggestions, or that it didn't answer. */
export interface YouTubeAnswer {
  videos?: YouTubeVideo[]
  more?: string | null
  suggestions?: string[]
  /** Channels a search found; the channel a channel's page is. */
  channels?: YouTubeChannel[]
  channel?: YouTubeChannel | null
  /** Playlists a search found; the playlist a playlist's page is. */
  playlists?: YouTubePlaylist[]
  playlist?: YouTubePlaylist | null
  /** A video's own page (Bilibili's): its uploader, its numbers, and the parts a series is played in. */
  detail?: YouTubeDetail | null
  /** The signed-in player's own page (Bilibili's 我的): their level, coins and numbers. */
  profile?: YouTubeProfile | null
  /** The player's 动态: the cards of the feed of the UP 主s they follow. */
  dynamics?: YouTubeDynamic[]
  /** The player's messages (the rows of the tab asked for), and how many of each kind are unread. */
  messages?: YouTubeMessage[]
  unread?: YouTubeUnread
  /** A video's comments (its 评论区): the page of them asked for. */
  comments?: YouTubeComment[]
  /** A video's danmaku: the lines that scroll over its picture, each with when it comes. */
  danmaku?: YouTubeDanmaku[]
  /** Whether Bilibili took the line of danmaku the page sent. */
  sent?: boolean
  failed?: boolean
}

/** One line of a video's danmaku: when it comes (seconds into the video), what it says, its colour. */
export interface YouTubeDanmaku {
  at: number
  text: string
  color: string | null
}

/** One comment of a video's 评论区. */
export interface YouTubeComment {
  id: string
  name: string
  avatar: string | null
  /** "3小时前" */
  time: string
  text: string
  /** "8", "1.2万" */
  likes: string
  /** How many replies it has under it. */
  replies: number
}

/** The signed-in player's own page: who they are, their level and membership, their coins, and their numbers. */
export interface YouTubeProfile {
  mid: string
  name: string
  avatar: string | null
  /** 0 to 6, as Bilibili's badge shows it. */
  level: number
  /** Whether they're a 大会员. */
  vip: boolean
  /** "880.6" */
  coins: string
  /** "0.34" */
  bcoins: string
  sign: string
  videos: number
  following: number
  followers: number
}

/** One card of the player's 动态: who put it up and when, what they did, its words and its video or pictures. */
export interface YouTubeDynamic {
  id: string
  author: string
  avatar: string | null
  channelId?: string | null
  /** "刚刚", "5分钟前" */
  time: string
  /** "投稿了视频", "发布了动态" */
  action: string
  text: string
  video?: YouTubeVideo | null
  pictures: string[]
  forwards: string
  comments: string
  likes: string
}

/** One row of the player's messages. */
export interface YouTubeMessage {
  id: string
  kind: 'reply' | 'at' | 'like' | 'notice'
  name: string
  avatar: string | null
  time: string
  text: string
  unread: boolean
}

/** How many of the player's messages are unread, by kind. */
export interface YouTubeUnread {
  reply: number
  at: number
  like: number
  notice: number
}

/** A video's own page, as Bilibili's player and the details beside it show it. */
export interface YouTubeDetail {
  video: string
  title: string
  desc: string
  channel: string
  channelId?: string | null
  avatar?: string | null
  pubdate: string
  views: string
  likes: string
  coins: string
  favorites: string
  danmaku: string
  /** Its parts: a one-part video is a list of one; a series has each of its own. */
  parts: YouTubePart[]
}

/** One part of a video (a series' own episode): its number, its `cid` (what the page plays), its title and
 * length. */
export interface YouTubePart {
  page: number
  cid: number
  title: string
  duration: string
}

/** Text typed, or a key with a meaning of its own. */
export type TypedEvent = { text: string } | { key: 'backspace' | 'wordBackspace' | 'enter' | 'escape' }

/** Whether the player's own keyboard types into the field selected: no field selected (`off`), typing, or
 * the keys the game's (`paused`: the truck moved, or the field was selected while it moved). */
export type TypingState = 'off' | 'typing' | 'paused'

/** What the page asks of the game's route: a stop added (before a job's destination), the route to a place
 * alone (not on a job), or no more stops. */
export type RouteAction = 'addStop' | 'go' | 'end'

/** A route WestPlay planned (crates/host planner.rs): what it's planned for, its length (metres) and time
 * (seconds), both on the map's ground (the page gives them in the game's scale), and its line through its
 * map nodes (`[x, z]`, metres). */
export interface PlannedRoute {
  plan: 'fastest' | 'shortest' | 'smallRoads' | 'other'
  length: number
  seconds: number
  line: [number, number][]
}

/** How the game answered: done, or why not (`noAnswer`: no game to ask, or it didn't answer in time;
 * `elevated`: the game runs as administrator, so WestPlay can't see its route, a `followed` answer's). */
export type RouteAnswer =
  'done' | 'alreadyThere' | 'onJob' | 'full' | 'noSuchNode' | 'unsupported' | 'unexpected' | 'noAnswer' | 'elevated'

interface WebViewBridge {
  addEventListener(type: 'message', listener: (event: { data: HostMessage }) => void): void
  removeEventListener(type: 'message', listener: (event: { data: HostMessage }) => void): void
  postMessage(message: PageMessage): void
}

declare global {
  interface Window {
    chrome?: { webview?: WebViewBridge }
  }
}

/** The cabin page as a live demo in the companion's onboarding (`cabin.html?demo`): never the host's page,
 * even inside a WebView2 (the companion's own). See cabin/demo.ts. */
export const IS_DEMO = typeof location !== 'undefined' && new URLSearchParams(location.search).has('demo')

/** Whether the page runs inside the WestPlay host (and not in a plain browser preview, nor as the demo). */
export function isInHost(): boolean {
  return !IS_DEMO && window.chrome?.webview !== undefined
}

/**
 * Calls `listener` for every host message. Returns the unsubscribe function.
 *
 * Subscribing also tells the host the page is ready, so it sends the current state again: anything it
 * sent before this page (re)loaded was lost, and some state (like the truck) is only sent on change.
 */
export function subscribeHostMessages(listener: (message: HostMessage) => void): () => void {
  const webview = window.chrome?.webview
  if (!webview) return () => {}
  const onMessage = (event: { data: HostMessage }) => listener(event.data)
  webview.addEventListener('message', onMessage)
  webview.postMessage({ type: 'ready' })
  return () => webview.removeEventListener('message', onMessage)
}

/** Sends a message to the host (nothing happens in a plain browser preview). */
export function postToHost(message: PageMessage): void {
  window.chrome?.webview?.postMessage(message)
}

/** A truck with no identity (the SDK reports empty strings when there is no truck). */
export function hasTruck(truck: TruckInfo | null): truck is TruckInfo {
  return truck !== null && truck.brandId !== ''
}

/** The next traffic light ahead, as the game's own semaphore says it (crates/host traffic_lights.rs). */
export interface TrafficLight {
  state: 'red' | 'amber' | 'green' | 'sleep'
  /** How long this state lasts, seconds. */
  seconds: number
  /** How far ahead it is, metres. */
  distance: number
  /** Its place on the game's ground (x east, z south), metres. */
  x: number
  z: number
}

/** The trailer hitched (`westplay_shared::TrailerInfo`): its fields are empty when there is none. */
export interface TrailerInfo {
  /** Its model id, e.g. `trailer.scs.box`. */
  id: string
  brandId: string
  /** Display brand and model, localized by the game (a trailer it has no name for: empty). */
  brand: string
  name: string
  /** Its plate as the game shows it. */
  plate: string
  /** The SDK's own ids: `single`, `double`, `b-double`… and `dryvan`, `refrigerated`, `flatbed`… */
  chainType: string
  bodyType: string
}

/** A streaming site the web app shows (`westplay_host::webapp::Site`). */
export type WebAppSite = 'bilibili' | 'qqMusic' | 'netease' | 'douyin' | 'kugou'

/** What the WebApp app asks of its site's page (`westplay_host::webapp::Command`): the site to open, where the
 * page shows (the frame's pixels, under the page), and what it does. */

/** How the web app's page is: the site it shows, the page's own `<title>` (empty until it has one), whether it
 * can go back in its history, and whether a video of its is in full screen (the app covers the CarPlay area then,
 * WebAppScreen.tsx). */
export interface WebAppState {
  site: WebAppSite
  title: string
  canGoBack: boolean
  fullScreen: boolean
  /** Whether the page is on show in the app: it has said how wide it lays out, and the app has it in view.
   * The cabin page cuts itself open for the site only then (before that its own background covers the room,
   * so the game's own navigation doesn't show through while the site loads). */
  shown: boolean
}

/** What the web app's page plays, as the site tells the system (its Media Session) and the page's own lyrics
 * say (`westplay_host::webapp::Media`). */
export interface WebAppMedia {
  title: string
  artist: string
  /** The cover's URL (`null`: none). */
  artwork: string | null
  playing: boolean
  /** The line of the song's words it's at (`null`: the page has none). */
  lyric: string | null
}

/** What the WebApp app asks of its site's page (`westplay_host::webapp::Command`): the site to open, where the
 * page shows (the frame's pixels, under the page), and what it does. */
export type WebAppCommand =
  | { action: 'open'; site: WebAppSite }
  | { action: 'show'; x: number; y: number; width: number; height: number }
  | { action: 'hide' }
  | { action: 'reload' }
  | { action: 'home' }
  | { action: 'back' }
  | { action: 'playPause' }
  | { action: 'next' }
  | { action: 'previous' }
  /** The cabin's volume slider, onto the page's own sound (0: silent). */
  | { action: 'setVolume'; volume: number }
  /** The cab's own quiet over the page's sound while the truck reverses: held while a level, `null` letting the
   * page's own volume back (the one volume a page that keeps its own volume takes). */
  | { action: 'duckVolume'; volume: number | null }
  /** The cabin's keyboard typing into the page's field: its text, whole. */
  | { action: 'setText'; text: string }
  /** The cabin's own input method's pick, into the page's field at the page's own caret, through the browser
   * itself (`Input.insertText`): the way into a field the page keeps to itself (Bilibili's own components). */
  | { action: 'insertText'; text: string }
  /** Its return key (a search's Enter). */
  | { action: 'enter' }
  | { action: 'close' }

/** How the web app's page is: the site it shows, the page's own `<title>` (empty until it has one), whether it
 * can go back in its history, and whether a video of its is in full screen (the app covers the CarPlay area then,
 * WebAppScreen.tsx). */

/** The bits of `Telemetry.engineKnown`: which engine, consumable and distance channels the game answered. */

export const ENGINE = {
  air: 1 << 0,
  adblue: 1 << 1,
  water: 1 << 2,
  odometer: 1 << 3,
} as const

/** The bits of `Telemetry.wearKnown`: which wear and damage channels the game answered (the host's `WEAR_*`). */
export const WEAR = {
  engine: 1 << 0,
  transmission: 1 << 1,
  cab: 1 << 2,
  chassis: 1 << 3,
  wheels: 1 << 4,
  trailerChassis: 1 << 5,
  trailerWheels: 1 << 6,
  cargo: 1 << 7,
} as const

/** Writes a line into the host's log (`app.log`), from the cabin page: nothing happens in a plain browser, or
 * where there's no window at all (a test). */
export function logToHost(text: string): void {
  if (typeof window === 'undefined') return
  postToHost({ type: 'log', text })
}

export type QqMusicRepeat = AppleMusicRepeat

export type QqMusicQuestion =
  | { ask: 'home' }
  /** The official charts (官方歌单): the layer's groups of lists. */
  | { ask: 'top' }
  | { ask: 'search'; query: string }
  | { ask: 'collection'; id: string; kind?: AppleMusicItem['kind']; name?: string; by?: string; image?: string | null }
  | { ask: 'lyrics'; id: string }
export interface QqMusicAnswers {
  home: AppleMusicPage
  top: AppleMusicPage
  search: AppleMusicResults
  collection: AppleMusicCollection
  lyrics: Lyrics
}

export type QqMusicPlayback = AppleMusicPlayback

export type QqMusicCommand =
  | { action: 'start' }
  | { action: 'ask'; id: number; question: QqMusicQuestion }
  | { action: 'play'; id?: string; songs?: string[]; index?: number; shuffle?: boolean }
  | { action: 'pause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; seconds: number }
  | {
      action: 'volume'
      percent: number
      /** A fade's (the cab's own quiet): not the player's choice, so the host doesn't keep it. */
      transient?: boolean
    }
  | { action: 'shuffle'; on: boolean }
  | { action: 'repeat'; mode: QqMusicRepeat }
  | { action: 'jump'; offset: number }
  | { action: 'favorite'; id: string; on: boolean }
  | { action: 'add'; id: string }
  /** Open the site's own sign-in window on the desktop. */
  | { action: 'signIn' }

/** What the Radio app asks of live radio's web player (`westplay_host::radio::Command`). */

export type RadioCommand =
  | { action: 'play'; track?: RadioTrack; requestId?: number }
  | { action: 'pause'; requestId?: number }
  | { action: 'volume'; percent: number }
  | { action: 'ask'; id: number; question: RadioQuestion }

export type RadioQuestion =
  | { ask: 'radioHome' }
  | { ask: 'radioList'; locationId: number; locationTypeId: number; categoryId: number; pageNum: number }
  | { ask: 'radioDetail'; radioId: number }
  | { ask: 'radioSchedule'; radioId: number }

export interface RadioTrack {
  id: string
  title: string
  artist: string
  artwork?: string
  radioId: number
}

export interface RadioPlayback {
  title: string
  artist: string
  artwork?: string
  status: 'loading' | 'playing' | 'paused' | 'failed'
  radioId: number
  requestId?: number
  error?: string
}

/** Which of the sound's channels a truck speaker plays (the host's `speakers::Channel`). */
export type SpeakerChannel = 'left' | 'right' | 'both'

/** A truck's own speaker, placed in its cab (the host's `speakers::Speaker`): what it plays and where it sits, in
 * the SDK's cab space (metres, x right, y up, z back from the cab's middle). */
export interface Speaker {
  /** What it plays: from this many Hz up to that many (0: no limit that way). */
  band: [number, number]
  /** How loud, against the others (dB). */
  level: number
  channel: SpeakerChannel
  /** Where it is in the cab (cab space, metres). */
  at: [number, number, number]
  /** How much later it plays than the rest (ms). */
  delay: number
}

/** The driver's head in the cab, and which way it faces (the host's `speakers::Head`): turns, heading left of
 * forward, pitch up. */
export interface CabHead {
  x: number
  y: number
  z: number
  heading: number
  pitch: number
}
/** A command for NetEase Cloud Music's hidden player (`crates/host/src/netease.rs`): start it (the page's layer
 * does on its first use), sign its account out, or a question its page's layer asks (`ask`), answered in
 * `neteaseAnswer` with the same `id`. */
export type NeteaseCommand =
  | { action: 'start' }
  | { action: 'signOut' }
  | { action: 'ask'; id: number; question: NeteaseQuestion }
  | { action: 'playPause' }
  | { action: 'next' }
  | { action: 'previous' }
  | { action: 'seek'; at: number }
  | { action: 'favorite'; id: number; on: boolean }
  | { action: 'signIn' }

/** A question the NetEase app's layer asks its hidden player (`apps/netease/layer.ts`): `ask` names what is
 * wanted, and the rest is that question's own (`search`'s `query`, `lyric`'s `id`). */
export interface NeteaseQuestion {
  ask: string
  id?: number
  query?: string
  [more: string]: unknown
}