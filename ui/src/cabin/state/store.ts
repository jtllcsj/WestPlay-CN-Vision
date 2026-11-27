import { create } from 'zustand'
import { createJSONStorage, persist } from 'zustand/middleware'
import {
  type Equalizer,
  FLAT,
  type HostMessage,
  type GameWeather,
  hasTruck,
  type JobInfo,
  IS_DEMO,
  type KeybindAction,
  isInHost,
  postToHost,
  subscribeHostMessages,
  type Telemetry,
  type TrafficLight,
  toEqualizer,
  type TrailerInfo,
  type TruckInfo,
} from '../../shared/host'
import type { Maneuver, TurnKind } from '../maps/guidance'
import {
  receiveAnswer as receiveAppleMusicAnswer,
  receivePlayback as receiveAppleMusicPlayback,
  useAppleMusic,
} from '../apps/music/appleMusic'
import {
  receiveAnswer as receiveQqMusicAnswer,
  receivePlayback as receiveQqMusicPlayback,
  receiveState as receiveQqMusicState,
} from '../apps/qq/qqMusic'
import {
  receiveAnswer as receiveSpotifyAnswer,
  receivePlayback,
  receiveState as receiveSpotifyState,
} from '../apps/spotify/spotify'
import {
  receiveWebApp,
  receiveWebAppLyricAt,
  receiveWebAppLyrics,
  receiveWebAppMedia,
  receiveWebAppTyping,
  receiveWebAppVolume,
} from '../apps/web/webapp'
import { receiveAnswer, receiveState, useYouTube } from '../apps/youtube/youtube'
import { receiveCab } from '../apps/netease/spatial'
import { receiveFolder, receivePlayback as receivePlayerPlayback } from '../apps/player/player'
import { receiveRadioAnswer } from '../apps/radio/api'
import { receiveRadioPlayback } from '../apps/radio/radio'
import {
  receiveClosed as receiveSafariClosed,
  receiveDialog as receiveSafariDialog,
  receiveLetGo as receiveSafariLetGo,
  receiveNoDownload as receiveSafariNoDownload,
  receiveOpened as receiveSafariOpened,
  receivePicture as receiveSafariPicture,
  receivePlaying as receiveSafariPlaying,
  receiveTab as receiveSafariTab,
} from '../apps/safari/safari'
import {
  receiveAnswer as receiveYouTubeMusicAnswer,
  receivePlayback as receiveYouTubeMusicPlayback,
  receiveState as receiveYouTubeMusicState,
} from '../apps/youtubeMusic/youtubeMusic'
import { useKeyboard } from '../keyboard/store'
import { receiveMapFolder } from '../maps/mapData'
import { receiveDetours, receiveFollowed, receiveRouteAnswer, receiveRoutes } from '../maps/routeChange'
import { type Point, restLine, type RouteNode, routeSpans } from '../maps/route'
import { DEFAULT_WALLPAPER_ID } from '../wallpapers'
import { settingsShown } from './wallpaperRotation'
import { isReleasedApp } from './apps'
import type { Appearance, AppearanceMode } from './appearance'
import { afterRunningTransition, animateNavigation, launchFor, type LaunchOrigin } from './launch'
import {
  INITIAL_NAVIGATION,
  navigate,
  type NavigationAction,
  type NavigationState,
  type Route,
  routeKey,
} from './navigation'
import { isIconStyle, setIcons, useLooks } from './looks'
import type { OemMode } from './oem'
import { INITIAL_POWER, power, type PowerEvent, type PowerState } from './power'
import { DEFAULT_REFRESH_RATE, isRefreshRate, type RefreshRate } from './refreshRate'
import { isScreenLayoutId, type ScreenLayoutId } from './screen'
import { DEFAULT_TEXT_SIZE, STEPS_ADDED_BELOW, toTextSize } from './textSize'

/**
 * The Dashboard's widgets the player can turn off (turn-by-turn always shows while there's a route). The two
 * newest — a picture of the player's own (a wallpaper's, chosen in Settings) and the truck's condition — come
 * off by default: they're the player's to turn on (owner 2026-11-03).
 */
export type DashboardWidget = 'nowPlaying' | 'destination' | 'weather' | 'photo' | 'vehicle'

export const DASHBOARD_WIDGETS: readonly DashboardWidget[] = [
  'nowPlaying',
  'destination',
  'weather',
  'photo',
  'vehicle',
]

/** Each widget as a player who never touched them has them: the first three on, the two newest off. */
export const DASHBOARD_WIDGET_DEFAULTS: Record<DashboardWidget, boolean> = {
  nowPlaying: true,
  destination: true,
  weather: true,
  photo: false,
  vehicle: false,
}

/**
 * Cabin state that the UI renders from. Telemetry lives in its own store (`useLive`) because it updates
 * up to 20 times a second; components subscribe to just the fields they need.
 */
interface CabinState {
  navigation: NavigationState
  appearanceMode: AppearanceMode
  wallpaperId: string
  /** A different wallpaper at each power-up (Settings › Wallpaper, state/wallpaperRotation.ts). */
  rotateWallpaper: boolean
  /** A wallpaper of its own for each look (Settings › Wallpaper): on, `wallpaperLight` and `wallpaperDark` are
   * shown in place of `wallpaperId`, which is left as it was for when it is off again. It and rotation don't go
   * together: switching one on switches the other off. */
  wallpapersByAppearance: boolean
  wallpaperLight: string
  wallpaperDark: string
  /** The Dashboard's widgets shown (Settings, Dashboard). */
  dashboardWidgets: Record<DashboardWidget, boolean>
  /** The picture the Dashboard's photo widget shows: a wallpaper of the gallery's (`''`: the wallpaper on
   * screen, so turning the widget on shows something at once). Settings › Dashboard picks it. */
  dashboardPhoto: string
  /** Album art on Now Playing and in Music (Settings, Now Playing): off, a plain note instead. */
  albumArt: boolean
  /** The Dashboard's map filling it while driving, the screen left alone (Settings › Dashboard; owner
   * 2026-11-03): off, the widgets always shown. */
  mapGrows: boolean
  /** The clock in 24-hour time (Settings › Date & Time; owner 2026-10-28): off, 12-hour with AM and PM. */
  hours24: boolean
  /** The apps' text size, a step of `TEXT_SIZES` (Settings › Appearance › Text Size). */
  textSize: number
  /** Every glass surface flat and solid, nothing behind it showing through (Settings › Appearance, as iOS's
   * Accessibility one; owner 2026-11-16: "make the sidebar and any liquid glass element flat"). */
  reduceTransparency: boolean
  /** The truck's own strip: kept (`auto`) or not, as set in the companion window (the host sends it). */
  oemMode: OemMode
  /** How often WestPlay hands the game a new frame (Settings › Display & Performance), 60 Hz by default: the
   * host's own say, kept with WestPlay's settings, and what the cabin page draws its map at. */
  refreshRate: RefreshRate
  /** The next traffic light shown on the map (Settings › Maps, where the owner put it: it draws on the map, so
   * it belongs with it, 2026-11-02): on by default. Off, the host stops reading the game's memory altogether
   * (crates/host traffic_lights.rs). */
  trafficLights: boolean
  /** The game's own mirror on the dashboard, through the hole the page cuts for it (Settings › Display &
   * Performance, owner 2026-11-03): on by default, as it shipped. Off, the dashboard draws nothing there and no
   * hole is cut, so its page stays whole. The page's own, as the text size is: the host keeps nothing of it. */
  mirrorOn: boolean
  /** The map's following camera zooms with the truck's speed — close in town, further out on the motorway, so the
   * road ahead keeps the same time (Settings › Maps, `maps/camera.ts`): on by default. Off, it follows at one zoom
   * for every speed. The page's own, as the text size is: the map is drawn here, so the host keeps nothing of it. */
  zoomWithSpeed: boolean
  /** Whether the map's buildings rise from it, as set in the companion window (the host sends it). */
  mapBuildings: boolean
  /** Whether its roads rise to their real heights, as set there too. */
  mapRoads: boolean
  /** The player's own tone curve (Settings › Equalizer), and whether it plays (Custom) or the speakers as tuned
   * (Recommended, flat): kept with WestPlay's settings, the host sends them. */
  equalizer: Equalizer
  equalizerCustom: boolean
  /** The truck's own speakers, each heard from where it sits in the cab (Settings › Sound). Off, the sound
   * plays as it is (stereo), through the tone and the cab's walls alone: kept with WestPlay's settings, the
   * host sends them. */
  spatial: boolean
  /** The screen's brightness (Settings › Appearance): the slider's place, 0 to 1, full at 1. The host scales the
   * light of everything on the screen by it (owner 2026-11-11: "real brightness. Not just a black overlay"). */
  brightness: number
  power: PowerState
  /**
   * The screen is handed back to the truck (its stock screen shows, with WestPlay's button to come back),
   * like CarPlay's car button. Not kept: WestPlay comes back with every power-up, like CarPlay reconnecting.
   * Never on WestPlay's tablet, which is no truck's screen: nothing would show there (owner 2026-11-11).
   */
  truckScreen: boolean
  navigate(action: NavigationAction): void
  setAppearanceMode(mode: AppearanceMode): void
  setWallpaper(id: string): void
  setRotateWallpaper(on: boolean): void
  setWallpapersByAppearance(on: boolean): void
  /** The wallpaper shown in `appearance` (Settings › Wallpaper, while `wallpapersByAppearance` is on). */
  setWallpaperFor(appearance: Appearance, id: string): void
  setDashboardWidget(widget: DashboardWidget, shown: boolean): void
  setAlbumArt(shown: boolean): void
  /** The Dashboard's photo widget's picture: a wallpaper's id, or `''` for the wallpaper on screen. */
  setDashboardPhoto(id: string): void
  setMapGrows(on: boolean): void
  setHours24(on: boolean): void
  setTextSize(step: number): void
  setReduceTransparency(on: boolean): void
  /** One band of the curve, set in Settings › Equalizer (choosing Custom): played by every player, and kept with
   * WestPlay's settings. */
  setEqualizerBand(band: keyof Equalizer, db: number): void
  /** Custom (the curve plays) or Recommended (flat), the curve kept either way. */
  chooseEqualizer(custom: boolean): void
  /** The curve flat again. */
  resetEqualizer(): void
  setSpatial(on: boolean): void
  setBrightness(level: number): void
  setOemMode(mode: OemMode): void
  /** The player's choice, in Settings: applied, and kept with WestPlay's settings. */
  chooseOemMode(mode: OemMode): void
  /** The host's own value: applied as it says it (the settings message). */
  setRefreshRate(rate: RefreshRate): void
  /** The player's choice, in Settings › Display & Performance: applied, and kept with WestPlay's settings (the
   * host publishes WestPlay's frames at it from then on). */
  chooseRefreshRate(rate: RefreshRate): void
  /** The next traffic light shown or not (Settings › Maps), the host told either way. */
  setTrafficLights(on: boolean): void
  /** The game's mirror on the dashboard or not (Settings › Display & Performance): applied here, kept with the
   * page's own settings (`MirrorWidget` cuts the hole only while it's on). */
  setMirrorOn(on: boolean): void
  /** The map follows the speed or not (Settings › Maps): the page's own, kept with its settings. */
  setZoomWithSpeed(on: boolean): void
  powerEvent(event: PowerEvent): void
  setTruckScreen(shown: boolean): void
}

/** What survives a restart of the app: where you were and your choices, never the power state. */
type Persisted = Pick<
  CabinState,
  | 'navigation'
  | 'appearanceMode'
  | 'wallpaperId'
  | 'rotateWallpaper'
  | 'wallpapersByAppearance'
  | 'wallpaperLight'
  | 'wallpaperDark'
  | 'dashboardWidgets'
  | 'dashboardPhoto'
  | 'albumArt'
  | 'mapGrows'
  | 'hours24'
  | 'textSize'
  | 'reduceTransparency'
  | 'trafficLights'
  | 'zoomWithSpeed'
  | 'brightness'
  | 'mirrorOn'
>

const ALL_WIDGETS = DASHBOARD_WIDGET_DEFAULTS

export const useCabin = create<CabinState>()(
  persist(
    (set) => ({
      navigation: INITIAL_NAVIGATION,
      appearanceMode: 'auto',
      wallpaperId: DEFAULT_WALLPAPER_ID,
      rotateWallpaper: false,
      wallpapersByAppearance: false,
      wallpaperLight: DEFAULT_WALLPAPER_ID,
      wallpaperDark: DEFAULT_WALLPAPER_ID,
      dashboardWidgets: { ...ALL_WIDGETS },
      dashboardPhoto: '',
      albumArt: true,
      mapGrows: true,
      hours24: true,
      textSize: DEFAULT_TEXT_SIZE,
      reduceTransparency: false,
      trafficLights: true,
      oemMode: 'auto',
      refreshRate: DEFAULT_REFRESH_RATE,
      zoomWithSpeed: true,
      mirrorOn: true,
      mapBuildings: true,
      mapRoads: true,
      equalizer: FLAT,
      equalizerCustom: false,
      spatial: true,
      brightness: 1,
      power: INITIAL_POWER,
      navigate: (action) => set((state) => ({ navigation: navigate(state.navigation, action) })),
      setAppearanceMode: (appearanceMode) => set({ appearanceMode }),
      // The two don't go together: switching one on switches the other off.
      setRotateWallpaper: (rotateWallpaper) =>
        set(rotateWallpaper ? { rotateWallpaper, wallpapersByAppearance: false } : { rotateWallpaper }),
      setWallpapersByAppearance: (wallpapersByAppearance) => {
        const now = useCabin.getState()
        // The first time, the pair starts as the wallpaper on screen: switching it on changes nothing until the
        // player picks one for each. A pair already chosen stays as it is.
        const untouched = now.wallpaperLight === DEFAULT_WALLPAPER_ID && now.wallpaperDark === DEFAULT_WALLPAPER_ID
        const pair = untouched ? { wallpaperLight: now.wallpaperId, wallpaperDark: now.wallpaperId } : {}
        set(
          wallpapersByAppearance
            ? { wallpapersByAppearance, rotateWallpaper: false, ...pair }
            : { wallpapersByAppearance },
        )
      },
      setSpatial: (on) => {
        set({ spatial: on })
        if (isInHost()) postToHost({ type: 'spatial', on })
      },
      setWallpaperFor: (appearance, id) => set(appearance === 'dark' ? { wallpaperDark: id } : { wallpaperLight: id }),
      setWallpaper: (id) => {
        set({ wallpaperId: id })
        // Kept with WestPlay's settings, where the companion's Customize sees it.
        if (isInHost()) postToHost({ type: 'wallpaper', id })
      },
      setDashboardWidget: (widget, shown) =>
        set((state) => ({ dashboardWidgets: { ...state.dashboardWidgets, [widget]: shown } })),
      setAlbumArt: (albumArt) => set({ albumArt }),
      setDashboardPhoto: (dashboardPhoto) => set({ dashboardPhoto }),
      setMapGrows: (mapGrows) => set({ mapGrows }),
      setHours24: (hours24) => set({ hours24 }),
      setTextSize: (step) => set({ textSize: toTextSize(step) }),
      setReduceTransparency: (reduceTransparency) => set({ reduceTransparency }),
      setEqualizerBand: (band, db) => {
        const equalizer = toEqualizer({ ...useCabin.getState().equalizer, [band]: db })
        set({ equalizer, equalizerCustom: true })
        if (isInHost()) postToHost({ type: 'equalizer', custom: true, ...equalizer })
      },
      chooseEqualizer: (custom) => {
        set({ equalizerCustom: custom })
        if (isInHost()) postToHost({ type: 'equalizer', custom, ...useCabin.getState().equalizer })
      },
      resetEqualizer: () => {
        set({ equalizer: FLAT })
        if (isInHost()) postToHost({ type: 'equalizer', custom: useCabin.getState().equalizerCustom, ...FLAT })
      },
      setBrightness: (level) => set({ brightness: Number.isFinite(level) ? Math.max(0, Math.min(1, level)) : 1 }),
      setOemMode: (oemMode) => set({ oemMode }),
      chooseOemMode: (oemMode) => {
        set({ oemMode })
        if (isInHost()) postToHost({ type: 'truckStrip', strip: oemMode })
      },
      setRefreshRate: (refreshRate) => {
        set({ refreshRate })
        // The page's map is drawn at the same rate the host publishes its frames at (maps/frameCap.ts).
      },
      chooseRefreshRate: (refreshRate) => {
        useCabin.getState().setRefreshRate(refreshRate)
        if (isInHost()) postToHost({ type: 'refreshRate', rate: refreshRate })
      },
      setTrafficLights: (trafficLights) => {
        set({ trafficLights })
        // Off, the host stops reading the game's memory at all (traffic_lights.rs): not just hidden.
        if (isInHost()) postToHost({ type: 'trafficLights', on: trafficLights })
      },
      setZoomWithSpeed: (zoomWithSpeed) => set({ zoomWithSpeed }),
      setMirrorOn: (mirrorOn) => set({ mirrorOn }),
      powerEvent: (event) =>
        set((state) => {
          const next = power(state.power, event)
          return { power: next, truckScreen: next.phase === 'off' ? false : state.truckScreen }
        }),
      truckScreen: false,
      setTruckScreen: (truckScreen) =>
        set({ truckScreen: truckScreen && useLive.getState().screenLayout !== 'tablet' }),
    }),
    {
      // The onboarding's demo keeps its own: it never changes the player's.
      name: IS_DEMO ? 'westplay.cabin.demo' : 'westplay.cabin',
      // 2: Text Size's steps moved larger (2026-10-24): a step kept from before means another size now, so each
      // player starts on the new default. 3: two smaller steps added below them (the same day): a step kept is the
      // same size two steps further on.
      version: 3,
      migrate: (stored, version) => {
        const kept = (stored ?? {}) as Partial<Persisted>
        if (version < 2) return { ...kept, textSize: DEFAULT_TEXT_SIZE }
        if (version < 3 && typeof kept.textSize === 'number') {
          return { ...kept, textSize: kept.textSize + STEPS_ADDED_BELOW }
        }
        return kept
      },
      storage: createJSONStorage(() => localStorage),
      partialize: (state): Persisted => ({
        navigation: state.navigation,
        appearanceMode: state.appearanceMode,
        wallpaperId: state.wallpaperId,
        rotateWallpaper: state.rotateWallpaper,
        wallpapersByAppearance: state.wallpapersByAppearance,
        wallpaperLight: state.wallpaperLight,
        wallpaperDark: state.wallpaperDark,
        dashboardWidgets: state.dashboardWidgets,
        dashboardPhoto: state.dashboardPhoto,
        albumArt: state.albumArt,
        mapGrows: state.mapGrows,
        hours24: state.hours24,
        textSize: state.textSize,
        reduceTransparency: state.reduceTransparency,
        trafficLights: state.trafficLights,
        zoomWithSpeed: state.zoomWithSpeed,
        mirrorOn: state.mirrorOn,
        brightness: state.brightness,
      }),
      // Stored data comes from an earlier version of WestPlay (or was edited): keep only what is valid.
      merge: (stored, current) => ({ ...current, ...sanitize(stored as Partial<Persisted> | undefined) }),
    },
  ),
)

function sanitize(stored: Partial<Persisted> | undefined): Partial<Persisted> {
  if (!stored) return {}
  const result: Partial<Persisted> = {}
  if (stored.navigation && isRoute(stored.navigation.route) && Array.isArray(stored.navigation.recents)) {
    const { route, recents, base } = stored.navigation
    result.navigation = {
      route,
      recents: recents.filter(isReleasedApp),
      // Saved before WestPlay remembered it: the screen shown, or the Dashboard (where it starts).
      base: base === 'home' || base === 'dashboard' ? base : route.kind === 'home' ? 'home' : 'dashboard',
    }
  }
  if (stored.appearanceMode === 'auto' || stored.appearanceMode === 'dark' || stored.appearanceMode === 'light') {
    result.appearanceMode = stored.appearanceMode
  }
  // Any id: one of the player's wallpapers is only known once the host lists them (one gone shows the default).
  if (typeof stored.wallpaperId === 'string') result.wallpaperId = stored.wallpaperId
  if (typeof stored.rotateWallpaper === 'boolean') result.rotateWallpaper = stored.rotateWallpaper
  if (typeof stored.wallpapersByAppearance === 'boolean') result.wallpapersByAppearance = stored.wallpapersByAppearance
  if (typeof stored.wallpaperLight === 'string') result.wallpaperLight = stored.wallpaperLight
  if (typeof stored.wallpaperDark === 'string') result.wallpaperDark = stored.wallpaperDark
  // Rotation and a pair of wallpapers don't go together: saved with both on, the pair is the one kept.
  if (result.wallpapersByAppearance === true) result.rotateWallpaper = false
  if (typeof stored.dashboardWidgets === 'object' && stored.dashboardWidgets !== null) {
    const widgets = stored.dashboardWidgets as Partial<Record<DashboardWidget, unknown>>
    // A widget the stored data doesn't name (one added since it was saved) takes its own default: the two newest
    // come off, where the first three came on.
    result.dashboardWidgets = Object.fromEntries(
      DASHBOARD_WIDGETS.map((widget) => [
        widget,
        typeof widgets[widget] === 'boolean' ? widgets[widget] : DASHBOARD_WIDGET_DEFAULTS[widget],
      ]),
    ) as Record<DashboardWidget, boolean>
  }
  // The picture the Dashboard's photo widget shows: a wallpaper's id, or `plain` (one of its own).
  if (typeof stored.dashboardPhoto === 'string') result.dashboardPhoto = stored.dashboardPhoto
  if (typeof stored.albumArt === 'boolean') result.albumArt = stored.albumArt
  if (typeof stored.mapGrows === 'boolean') result.mapGrows = stored.mapGrows
  if (typeof stored.hours24 === 'boolean') result.hours24 = stored.hours24
  if (stored.textSize !== undefined) result.textSize = toTextSize(stored.textSize)
  if (typeof stored.reduceTransparency === 'boolean') result.reduceTransparency = stored.reduceTransparency
  if (typeof stored.trafficLights === 'boolean') result.trafficLights = stored.trafficLights
  if (typeof stored.zoomWithSpeed === 'boolean') result.zoomWithSpeed = stored.zoomWithSpeed
  if (typeof stored.mirrorOn === 'boolean') result.mirrorOn = stored.mirrorOn
  if (typeof stored.brightness === 'number' && Number.isFinite(stored.brightness)) {
    result.brightness = Math.max(0, Math.min(1, stored.brightness))
  }
  return result
}

function isRoute(route: unknown): route is Route {
  if (typeof route !== 'object' || route === null) return false
  const { kind, app } = route as { kind?: unknown; app?: unknown }
  return kind === 'home' || kind === 'dashboard' || (kind === 'app' && isReleasedApp(app))
}

/** Live data from the game. */
export interface LiveState {
  telemetry: Telemetry | null
  truck: TruckInfo | null
  /** The player's job (`null`: none). */
  job: JobInfo | null
  /** The trailer hitched (`null`: none; its fields are empty when there is one it doesn't name). */
  trailer: TrailerInfo | null
  /** The kind of screen the game draws WestPlay on; `null` until it has drawn one. */
  screenLayout: ScreenLayoutId | null
  /** The cities whose garages the player owns, and their headquarters' (none until the host has read their
   * save). */
  ownedGarages: string[]
  headquarters: string | null
  /** The player plays on a map by HVT, which WestPlay doesn't allow: the game's language to say so in.
   * `null`: not (or not known yet). */
  hvtMap: { language: string | null } | null
  /** The game's language (its `g_lang_init`, e.g. `tr_tr`; `null` until the host has read it). */
  gameLanguage: string | null
  /** The language the player chose in the companion, over the game's (a game code); `null`: the game's. */
  chosenLanguage: string | null
  /** The player's profile has the game's time zones on (its `g_time_zones`): the clock is the time where
   * the truck is (state/timeZone.ts). */
  timeZones: boolean
  /** The game's fatigue simulation (its `uset g_fatigue`): with it off nothing forces a rest, and the map shows
   * no countdown (§69). */
  fatigueSimulation: boolean
  /** The player's profile is in miles and mph (its `g_mph`, Options › Regional settings): every speed and
   * distance shown in them; else kilometres and km/h. */
  mph: boolean
  /** How far the clock where the truck is runs from the game's time, minutes: its country's time zone with
   * the profile's time zones on, else 0 (state/timeZone.ts). Every clock shown adds it (`cabClock`). */
  clockShift: number
  /** The route the game's GPS plans (`null`: none): its map nodes, each one's distance left to the
   * destination (as the game counts distances), the ways through its junctions and the turns there (by the
   * node each starts at, once the host has found them), how far right of its way it runs at its nodes on
   * roads (on the carriageway it drives on), and the node ahead of the truck along it. */
  route: {
    id: number
    nodes: RouteNode[]
    /** The next coach stop's map node index. Without it the whole remaining route is active. */
    activeEnd?: number | null
    /** Map node indices of the remaining intermediate coach stops. */
    coachStops?: readonly number[]
    distances: number[]
    /** How high each node is in the game, metres. */
    heights: number[]
    paths: ReadonlyMap<number, Point[]>
    maneuvers: Maneuver[]
    shifts: ReadonlyMap<number, number>
  } | null
  routeNext: number
  /** The player's stops along the route: their map nodes' indices in it, ascending. */
  routeStops: readonly number[]
  /** The routes offered for the `routes` request `id` that the host has read so far, drawn as the game's route is, by
   * their place among them. */
  offeredLines: { id: number; lines: (Point[] | undefined)[] } | null
  /** The game's weather (`null`: not known). */
  weather: GameWeather | null
  /** Whether the game's window shows, in front or not (no game: as if it did). */
  gameShown: boolean
  /** The game's menu gone while it's still paused (the music comes back then, not 1.5 s later). */
  menuGone: boolean
  /** The keybind last pressed in the game, and how many so far (each press a new one). */
  keybind: { action: KeybindAction; count: number } | null
  /** Whether the game runs under TruckersMP. */
  truckersmp: boolean
  /** Which game it runs (`null`: not known): only that game's servers are the player's. */
  truckersmpGame: 'ets2' | 'ats' | null
  /** Under TruckersMP, the players around the player from its SDK, as the host last said (`null`: none). */
  nearby: Nearby | null
  /** The music's spectrum as the host last said it (Settings › Equalizer's, `HostMessage` `spectrum`). */
  spectrum: readonly number[]
  /** The host draws the screen's cursor (`cursor` message): the page draws none of its own then. */
  cursorByHost: boolean
  /** The next traffic light ahead, as the host reads it from the game (`null`: none). */
  trafficLight: TrafficLight | null
}

/** TruckersMP's players around the player, from its SDK: its game server's id, the player's own number on it,
 * the others (number, world x and z, heading in turns), and when it came (page clock, ms). */
export interface Nearby {
  server: number
  local: number
  players: [id: number, x: number, z: number, heading: number][]
  at: number
}

export const useLive = create<LiveState>()(() => ({
  telemetry: null,
  truck: previewTruck(),
  job: null,
  trailer: null,
  screenLayout: previewLayout(),
  ownedGarages: [],
  headquarters: null,
  hvtMap: null,
  gameLanguage: null,
  chosenLanguage: null,
  timeZones: false,
  fatigueSimulation: false,
  mph: false,
  clockShift: 0,
  route: null,
  routeNext: 0,
  routeStops: [],
  offeredLines: null,
  weather: null,
  gameShown: true,
  menuGone: false,
  keybind: null,
  truckersmp: false,
  truckersmpGame: null,
  nearby: null,
  spectrum: [],
  cursorByHost: false,
  trafficLight: null,
}))

/** Plain browser previews have no game to report the screen: `?layout=scania-2025` stands in for it. */
/** In a browser, `?truck=scania.s_2016` drives that truck (its screen profile). */
function previewTruck(): TruckInfo | null {
  if (isInHost()) return null
  const model = new URLSearchParams(location.search).get('truck')
  if (!model) return null
  const brandId = model.split('.')[0]
  return { brandId, brand: brandId, id: `vehicle.${model}`, name: model }
}

function previewLayout(): ScreenLayoutId | null {
  if (isInHost()) return null
  const layout = new URLSearchParams(location.search).get('layout')
  return isScreenLayoutId(layout) ? layout : null
}

/**
 * Feeds host messages into the stores, and tells the host whether WestPlay is open on the truck's screen
 * (as the page connects, and whenever it opens or closes). Returns the unsubscribe function; call once at
 * startup. The power sequence follows the truck's electrics (and only changes when they change).
 */
export function connectHost(now: () => number = () => performance.now()): () => void {
  const stopMessages = subscribeHostMessages(receiveHostMessage(now))
  const report = () => postToHost({ type: 'westplay', open: !useCabin.getState().truckScreen })
  report()
  // The traffic lights' setting as kept: the host reads the game's memory only while it's on.
  postToHost({ type: 'trafficLights', on: useCabin.getState().trafficLights })
  // The screen's brightness, for the host to scale the frame's light by: as the page connects, and as it changes.
  const reportBrightness = () => postToHost({ type: 'brightness', level: useCabin.getState().brightness })
  reportBrightness()
  const stopReports = useCabin.subscribe((state, previous) => {
    if (state.truckScreen !== previous.truckScreen) report()
    if (state.brightness !== previous.brightness) reportBrightness()
  })
  // The field selected, for the player's own keyboard to type into (the host passes it to the game plugin).
  const reportSelection = () => postToHost({ type: 'typing', selection: useKeyboard.getState().selection })
  reportSelection()
  const stopSelections = useKeyboard.subscribe((state, previous) => {
    if (state.selection !== previous.selection) reportSelection()
  })
  return () => {
    stopMessages()
    stopReports()
    stopSelections()
  }
}

function receiveHostMessage(now: () => number): (message: HostMessage) => void {
  return (message) => {
    switch (message.type) {
      case 'telemetry': {
        const previous = useLive.getState().telemetry
        useLive.setState({ telemetry: message.data })
        if (previous?.electricEnabled !== message.data.electricEnabled) {
          // The tablet isn't the truck's screen: no brand as it starts.
          const truckScreen = useLive.getState().screenLayout !== 'tablet'
          useCabin
            .getState()
            .powerEvent({ type: 'electrics', on: message.data.electricEnabled, at: now(), truckScreen })
        }
        break
      }
      case 'webApp':
        receiveWebApp(message.state)
        break
      case 'webAppMedia':
        receiveWebAppMedia(message.media)
        break
      case 'webAppLyrics':
        receiveWebAppLyrics(message.lyrics)
        break
      case 'webAppLyricAt':
        receiveWebAppLyricAt(message.at)
        break
      case 'webAppVolume':
        receiveWebAppVolume(message.volume)
        break
      case 'webAppTyping':
        receiveWebAppTyping(message.on, message.text, message.selection)
        break
      case 'trafficLight':
        useLive.setState({ trafficLight: message.light })
        break
      case 'truck':
        useLive.setState({ truck: hasTruck(message.data) ? message.data : null })
        break
      case 'job':
        useLive.setState({ job: message.data.active ? message.data : null })
        break
      case 'trailer':
        // A trailer the game has nothing to say about (unhitched) comes with empty fields.
        useLive.setState({ trailer: message.data.id === '' ? null : message.data })
        break
      case 'garages':
        useLive.setState({ ownedGarages: message.owned, headquarters: message.headquarters })
        break
      case 'routeAnswer':
        receiveRouteAnswer(message.id, message.answer)
        break
      case 'routes':
        receiveRoutes(message.id, message.routes)
        break
      case 'offeredWays': {
        const known = useLive.getState().offeredLines
        const lines = known?.id === message.id ? [...known.lines] : []
        const nodes = message.nodes.map(([x, z, fx, fz]): RouteNode => ({ position: [x, z], forward: [fx, fz] }))
        lines[message.index] = restLine(routeSpans(nodes, new Map(message.paths), new Map(message.shifts)), -1)
        useLive.setState({ offeredLines: { id: message.id, lines } })
        break
      }
      case 'followed':
        receiveFollowed(message.id, message.follows, message.answer)
        break
      case 'detours':
        receiveDetours(message.id, message.seconds)
        break
      case 'typed':
        for (const event of message.events) useKeyboard.getState().typed(event)
        break
      case 'typingState':
        useKeyboard.setState({ physical: message.state })
        break
      case 'fatigue':
        useLive.setState({ fatigueSimulation: message.on })
        break
      case 'mapBlock':
        useLive.setState({
          hvtMap: message.hvt ? { language: message.language } : null,
          gameLanguage: message.language,
        })
        break
      case 'timeZones':
        useLive.setState({ timeZones: message.on })
        break
      case 'units':
        if (message.mph !== null) useLive.setState({ mph: message.mph })
        break
      case 'route':
        useLive.setState({
          route:
            message.id === null || message.nodes === null
              ? null
              : {
                  id: message.id,
                  nodes: message.nodes.map(([x, z, fx, fz]) => ({ position: [x, z], forward: [fx, fz] })),
                  activeEnd: message.activeEnd,
                  coachStops: message.coachStops,
                  distances: message.nodes.map((node) => node[4]),
                  heights: message.nodes.map((node) => node[5]),
                  paths: new Map(),
                  maneuvers: [],
                  shifts: new Map(),
                },
          routeNext: 0,
          routeStops: [],
        })
        break
      case 'routeStops':
        if (useLive.getState().route?.id === message.id) useLive.setState({ routeStops: message.stops })
        break
      case 'routePaths': {
        const route = useLive.getState().route
        if (route?.id !== message.id) break
        const maneuvers = message.maneuvers.map(([node, turn, angle, exit]) => ({
          node,
          turn: turn as TurnKind,
          angle,
          exit,
        }))
        const shifts = new Map(message.shifts ?? [])
        useLive.setState({ route: { ...route, paths: new Map(message.paths), maneuvers, shifts } })
        break
      }
      case 'routeNext':
        useLive.setState({ routeNext: message.next })
        break
      case 'weather':
        useLive.setState({ weather: message.weather })
        break
      case 'screen':
        // A layout this UI doesn't know yet (newer plugin) is treated like a screen without a profile.
        useLive.setState({ screenLayout: isScreenLayoutId(message.layout) ? message.layout : null })
        break
      case 'settings': {
        const cabin = useCabin.getState()
        cabin.setOemMode(message.truckStrip === 'off' ? 'off' : 'auto')
        // Older hosts send no refresh rate: the standard one.
        cabin.setRefreshRate(isRefreshRate(message.refreshRate) ? message.refreshRate : DEFAULT_REFRESH_RATE)
        if (message.wallpaper) useCabin.setState({ wallpaperId: message.wallpaper })
        // Chosen here before WestPlay kept it with its settings: kept now.
        else if (cabin.wallpaperId !== DEFAULT_WALLPAPER_ID) postToHost({ type: 'wallpaper', id: cabin.wallpaperId })
        // The page's first settings: the start's own power-up, a new wallpaper (state/wallpaperRotation.ts).
        settingsShown()
        useCabin.setState({
          equalizer: toEqualizer(message.equalizer),
          equalizerCustom: message.equalizerCustom === true,
          // No word of the speakers: on, as every version before the switch had them.
          spatial: message.spatial !== false,
          mapBuildings: message.buildings !== false,
          mapRoads: message.roads !== false,
        })
        useLive.setState({ chosenLanguage: typeof message.language === 'string' ? message.language : null })
        setIcons({
          iconSet: message.iconSet ?? null,
          iconStyle: isIconStyle(message.iconStyle) ? message.iconStyle : 'auto',
        })
        break
      }
      case 'spectrum':
        useLive.setState({ spectrum: message.bars })
        break
      case 'cursor':
        useLive.setState({ cursorByHost: message.host })
        break
      case 'customize':
        useLooks.setState({ custom: { wallpapers: message.wallpapers, iconSets: message.iconSets } })
        break
      case 'westplay':
        useCabin.getState().setTruckScreen(!message.open)
        break
      case 'youtube':
        // Which site reported: YouTube's player and Bilibili's send the same shape, and each one's
        // state is kept under its own key (a video one stopped is not the other's).
        receiveState(message.state, message.site ?? 'youtube')
        break
      case 'gameShown':
        useLive.setState({ gameShown: message.shown })
        break
      case 'menuGone':
        useLive.setState({ menuGone: message.gone })
        break
      case 'keybind':
        useLive.setState((live) => ({ keybind: { action: message.action, count: (live.keybind?.count ?? 0) + 1 } }))
        break
      case 'cab':
        // The truck's speakers, its driver's head and the camera, for the cabin page's own player: NetEase
        // Cloud Music's app draws itself here, so it takes its sound into the same graph from here rather
        // than from a page of its own (`apps/netease/spatial.ts`).
        receiveCab({
          speakers: message.speakers,
          head: message.head,
          outside: message.outside,
          distance: message.distance,
          equalizer: message.equalizer,
        })
        break
      case 'truckersmp':
        useLive.setState({
          truckersmp: message.on,
          truckersmpGame: message.game ?? null,
          ...(message.on ? {} : { nearby: null }),
        })
        break
      case 'nearby':
        useLive.setState({
          nearby:
            message.server === null || message.local === null
              ? null
              : { server: message.server, local: message.local, players: message.players, at: now() },
        })
        break
      case 'game':
        // The game closed: the screen goes off as with the truck's electrics (the kept apps go with it), and
        // the next game's first telemetry powers it on afresh.
        if (!message.running) {
          useLive.setState({ telemetry: null })
          useCabin.getState().powerEvent({ type: 'electrics', on: false, at: now() })
        }
        break
      case 'youtubeAccount':
        useYouTube.setState({ account: { signedIn: message.signedIn, avatar: message.avatar } })
        break
      case 'youtubeSignIn':
        useYouTube.setState({ signingIn: message.open })
        break
      case 'map':
        receiveMapFolder(message.folder, message.built)
        break
      case 'spotify':
        receiveSpotifyState(message.signedIn, message.playing, message.signingIn)
        break
      case 'spotifyAnswer':
        receiveSpotifyAnswer(message)
        break
      case 'spotifyPlayback':
        receivePlayback(message.playback)
        break
      case 'appleMusic':
        useAppleMusic.setState({
          signedIn: message.signedIn,
          playing: message.playing,
          signingIn: message.signingIn,
          avatar: message.avatar,
        })
        break
      case 'appleMusicAnswer':
        receiveAppleMusicAnswer(message)
        break
      case 'appleMusicPlayback':
        receiveAppleMusicPlayback(message.playback)
        break
      // QQ Music's own player (crates/host qq_music.rs): its state, its answers, what it plays.
      case 'qqMusic':
        receiveQqMusicState(message.signedIn, message.avatar)
        break
      case 'qqMusicAnswer':
        receiveQqMusicAnswer(message)
        break
      case 'qqMusicPlayback':
        receiveQqMusicPlayback(message.playback)
        break
      case 'musicFolder':
        receiveFolder(message.folder, message.songs)
        break
      case 'playerPlayback':
        receivePlayerPlayback(message.playback)
        break
      case 'radioAnswer':
        receiveRadioAnswer(message.id, message.radio, message.failed, message.error)
        break
      case 'radioPlayback':
        receiveRadioPlayback(message.playback)
        break
      case 'safariTab':
        receiveSafariTab(message.tab, message.state)
        break
      case 'safariOpened':
        receiveSafariOpened(message.tab, message.opener)
        break
      case 'safariClosed':
        receiveSafariClosed(message.tab)
        break
      case 'safariLetGo':
        receiveSafariLetGo(message.tab)
        break
      case 'safariDialog':
        receiveSafariDialog(message.tab, message.dialog)
        break
      case 'safariPicture':
        receiveSafariPicture(message.tab, message.version)
        break
      case 'safariNoDownload':
        receiveSafariNoDownload()
        break
      case 'safariPlaying':
        receiveSafariPlaying(message.playing)
        break
      case 'youtubeMusic':
        receiveYouTubeMusicState(message)
        break
      case 'youtubeMusicAnswer':
        receiveYouTubeMusicAnswer(message)
        break
      case 'youtubeMusicPlayback':
        receiveYouTubeMusicPlayback(message.playback)
        break
      case 'youtubeAnswer': {
        const { type: _, id, ...answer } = message
        receiveAnswer(id, answer)
        break
      }
    }
  }
}

let latestNavigation = 0

/**
 * Navigates the way a player does, animated: an app grows out of the icon that opened it (`origin`) and
 * shrinks back into its Home or sidebar icon when it closes; Home and the Dashboard trade places
 * (state/launch.ts).
 *
 * Like a touchscreen, every press counts and the last one is what you see: a press during an animation
 * finishes it, presses overtaken by a later one apply at once, and the last one animates.
 */
export function go(action: NavigationAction, origin: LaunchOrigin | null = null): void {
  const id = ++latestNavigation
  afterRunningTransition(() => {
    const cabin = useCabin.getState()
    const before = cabin.navigation.route
    const after = navigate(cabin.navigation, action).route
    // Overtaken by a later press, nothing to change, or Home and the Dashboard trading places (animated live
    // by CabinApp's `Screens`): no View Transition.
    const betweenBases = before.kind !== 'app' && after.kind !== 'app'
    if (id !== latestNavigation || routeKey(after) === routeKey(before) || betweenBases) {
      cabin.navigate(action)
      return
    }
    animateNavigation(launchFor(before, after, origin), () => useCabin.getState().navigate(action))
  })
}
