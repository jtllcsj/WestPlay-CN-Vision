import { type Backdrop, type Look, useWordsOver, wordsClass } from '../apps/music/contrast'
import { Crossfade } from '../apps/music/Crossfade'
import { VideoWidget } from '../apps/youtube/VideoWidget'
import { command, nextVideo, play, previousVideo, useYouTube } from '../apps/youtube/youtube'
import {
  LyricsGlyph,
  NextGlyph,
  NoteGlyph,
  PauseGlyph,
  PlayGlyph,
  PreviousGlyph,
  SpeakerGlyph,
} from '../apps/music/glyphs'
import {
  playOrPause as playOrPauseNetease,
  skip as skipNetease,
  useNetease,
  useNeteaseSource,
} from '../apps/netease/netease'
import { TrackSlide } from '../apps/music/TrackSlide'
import { useTrackTransition } from '../apps/music/useTrackTransition'
import { playOrPause, setVolume as setSpotifyVolume, skip, useSpotify, useSpotifySource } from '../apps/spotify/spotify'
import {
  playOrPause as playOrPauseAppleMusic,
  setVolume as setAppleMusicVolume,
  skip as skipAppleMusic,
  useAppleMusic,
  useAppleMusicSource,
} from '../apps/music/appleMusic'
import {
  playOrPause as playOrPauseYouTubeMusic,
  setVolume as setYouTubeMusicVolume,
  skip as skipYouTubeMusic,
  useYouTubeMusic,
  useYouTubeMusicSource,
} from '../apps/youtubeMusic/youtubeMusic'
import { useEffect, useState } from 'react'
import { playOrPause as playOrPausePlayer, skip as skipPlayer, usePlayer, usePlayerSource } from '../apps/player/player'
import { playOrPauseRadio, setVolume as setRadioVolume, skipRadio, useRadio, useRadioSource } from '../apps/radio/radio'
import {
  playOrPause as playOrPauseQqMusic,
  setVolume as setQqMusicVolume,
  skip as skipQqMusic,
  useQqMusic,
  useQqMusicSource,
} from '../apps/qq/qqMusic'
import type { MediaAction, MediaSource } from '../../shared/media'
import { useLook } from '../state/hooks'
import { go, useCabin } from '../state/store'
import { useMapData } from '../maps/mapData'
import { useLive } from '../state/store'
import { useText } from '../state/text'
import { APP_OF_SITE, cabinVolumeFor, command as webAppCommand, useWebApp, wordsAreSongs } from '../apps/web/webapp'
import { bodyDamage, hitched, percent, trailerDamage } from '../apps/vehicle/vehicle'
import { useWallpapers } from '../state/looks'
import { prepareWallpaper } from '../wallpapers'
import { nearestCity } from './destination'
import { photoView } from './photo'
import { useBilibiliSource, useNowPlaying, useWebAppSource } from './useNowPlaying'
import { weatherView } from './weather'
import { WeatherGlyph } from './WeatherGlyph'
import { EMPTY, useDestinationView, useGuided } from './widgetState'
import { cabClock } from '../../shared/host'

/**
 * Now Playing: what plays, the video YouTube plays first (VideoWidget), then the Spotify app's music, then
 * Apple Music's (the Music app), then YouTube Music's, then Player's; nothing playing, what played last
 * (nowPlaying.ts); none of them with anything, Not Playing. Turned off in Settings, its room goes to the others.
 */
export function NowPlayingWidget() {
  const shown = useCabin((state) => state.dashboardWidgets.nowPlaying)
  const { showing, bilibili, webApp, spotify, appleMusic, youtubeMusic, qqMusic, netease, player, radio } = useNowPlaying()
  return (
    <div className={`widget-slot${shown ? ' open' : ''}`}>
      {shown && showing === 'video' && <VideoWidget />}
      {/* Bilibili's video as a song: its cover, its title and its uploader, with the three buttons —
          the same card the music apps get (`useBilibiliSource`). */}
      {shown && showing === 'bilibili' && bilibili && <BilibiliWidget bilibili={bilibili} />}
      {shown && showing === 'webApp' && webApp && <WebAppWidget webApp={webApp} />}
      {shown && showing === 'spotify' && spotify && <SpotifyWidget spotify={spotify} />}
      {shown && showing === 'appleMusic' && appleMusic && <AppleMusicWidget appleMusic={appleMusic} />}
      {shown && showing === 'youtubeMusic' && youtubeMusic && <YouTubeMusicWidget youtubeMusic={youtubeMusic} />}
      {shown && showing === 'qqMusic' && qqMusic && <QqMusicWidget qqMusic={qqMusic} />}
      {shown && showing === 'netease' && netease && <NeteaseWidget netease={netease} />}
      {shown && showing === 'player' && player && <PlayerWidget player={player} />}
      {shown && showing === 'radio' && radio && <RadioWidget radio={radio} />}
      {shown && showing === null && <MusicWidget source={null} artwork={null} onControl={() => {}} />}
    </div>
  )
}

/**
 * Bilibili's video, drawn as the music apps are: its cover, its title and its uploader on the
 * Dashboard's card, with play or pause, next and previous — not its picture, which is what the
 * video app itself is for (owner 2026-11-25).
 */
function BilibiliWidget({ bilibili }: { bilibili: NonNullable<ReturnType<typeof useBilibiliSource>> }) {
  const playing = bilibili.source.status === 'playing'
  return (
    <MusicWidget
      source={bilibili.source}
      artwork={bilibili.artwork}
      onOpen={() => {
        useYouTube.setState({ returnToPlayer: true })
        go({ type: 'openApp', app: 'youtube' })
      }}
      onControl={(action) => {
        if (action === 'playPause') command({ action: playing ? 'pause' : 'play' }, 'bilibili')
        else if (action === 'next') {
          const next = nextVideo()
          if (next) play(next, false)
        } else if (action === 'previous') {
          const before = previousVideo()
          if (before) play(before, false)
        }
      }}
    />
  )
}

/** NetEase Cloud Music's (its own player in the cabin page): what plays, its cover and volume, and its three
 * buttons — the same card QQ Music's gets. */
function NeteaseWidget({ netease }: { netease: NonNullable<ReturnType<typeof useNeteaseSource>> }) {
  return (
    <MusicWidget
      source={netease.source}
      artwork={netease.artwork}
      onOpen={() => {
        useNetease.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'spotify' })
      }}
      onControl={(action) => {
        if (action === 'playPause') playOrPauseNetease()
        else if (action === 'next') skipNetease(1)
        else if (action === 'previous') skipNetease(-1)
      }}
    />
  )
}

/** Apple Music's music (the Music app's): what plays, its controls, opening Music on Now Playing. */
function AppleMusicWidget({ appleMusic }: { appleMusic: NonNullable<ReturnType<typeof useAppleMusicSource>> }) {
  return (
    <MusicWidget
      source={appleMusic.source}
      artwork={appleMusic.artwork}
      onOpen={() => {
        useAppleMusic.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'music' })
      }}
      // Apple Music's own words are its own page's (Now Playing's lyrics, untouched): the button opens
      // it there (owner 2026-10-30: the lyrics entry is for the three music apps, not a video's).
      onLyrics={() => {
        useAppleMusic.setState({ nowPlaying: true, lyrics: true, queue: false })
        go({ type: 'openApp', app: 'music' })
      }}
      volume={useAppleMusic((state) => state.playback?.volume ?? null)}
      onVolume={(level, done) => setAppleMusicVolume(level, done)}
      onControl={(action) => {
        if (action === 'playPause') playOrPauseAppleMusic()
        else if (action === 'next') skipAppleMusic(1)
        else if (action === 'previous') skipAppleMusic(-1)
      }}
    />
  )
}

/** YouTube Music's music: what plays, its controls, opening YouTube Music on Now Playing. */
function YouTubeMusicWidget({ youtubeMusic }: { youtubeMusic: NonNullable<ReturnType<typeof useYouTubeMusicSource>> }) {
  return (
    <MusicWidget
      source={youtubeMusic.source}
      artwork={youtubeMusic.artwork}
      onOpen={() => {
        useYouTubeMusic.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'youtubeMusic' })
      }}
      volume={useYouTubeMusic((state) => state.playback?.volume ?? null)}
      onVolume={(level, done) => setYouTubeMusicVolume(level, done)}
      onControl={(action) => {
        if (action === 'playPause') playOrPauseYouTubeMusic()
        else if (action === 'next') skipYouTubeMusic(1)
        else if (action === 'previous') skipYouTubeMusic(-1)
      }}
    />
  )
}

/** Player's music (the game's music folder): what plays, its controls, opening Player on Now Playing. */
function PlayerWidget({ player }: { player: NonNullable<ReturnType<typeof usePlayerSource>> }) {
  return (
    <MusicWidget
      source={player.source}
      artwork={player.artwork}
      onOpen={() => {
        usePlayer.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'player' })
      }}
      onControl={(action) => {
        if (action === 'playPause') playOrPausePlayer()
        else if (action === 'next') skipPlayer(1)
        else if (action === 'previous') skipPlayer(-1)
      }}
    />
  )
}

/** Live radio: what plays, its controls, opening Radio on Now Playing. */
function RadioWidget({ radio }: { radio: NonNullable<ReturnType<typeof useRadioSource>> }) {
  return (
    <MusicWidget
      source={radio.source}
      artwork={radio.artwork}
      onOpen={() => {
        useRadio.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'radio' })
      }}
      volume={useRadio((state) => state.volume)}
      onVolume={(level) => setRadioVolume(level)}
      onControl={(action) => {
        if (action === 'playPause') playOrPauseRadio()
        else if (action === 'next') skipRadio(1)
        else if (action === 'previous') skipRadio(-1)
      }}
    />
  )
}

/** WestPlay's own QQ Music app: what QQ plays, its cover, volume and controls, opening QQ on Now Playing. */
function QqMusicWidget({ qqMusic }: { qqMusic: NonNullable<ReturnType<typeof useQqMusicSource>> }) {
  return (
    <MusicWidget
      source={qqMusic.source}
      artwork={qqMusic.artwork}
      onOpen={() => {
        useQqMusic.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'youtubeMusic' })
      }}
      volume={useQqMusic((state) => state.playback?.volume ?? null)}
      onVolume={(level, done) => setQqMusicVolume(level, done)}
      onControl={(action) => {
        if (action === 'playPause') playOrPauseQqMusic()
        else if (action === 'next') skipQqMusic(1)
        else if (action === 'previous') skipQqMusic(-1)
      }}
    />
  )
}

/** The Spotify app's music: what Spotify plays, its controls, opening the Spotify app on Now Playing. */
function SpotifyWidget({ spotify }: { spotify: NonNullable<ReturnType<typeof useSpotifySource>> }) {
  return (
    <MusicWidget
      source={spotify.source}
      artwork={spotify.artwork}
      onOpen={() => {
        useSpotify.setState({ nowPlaying: true })
        go({ type: 'openApp', app: 'spotify' })
      }}
      volume={useSpotify((state) => state.playback?.volume ?? null)}
      onVolume={(level, done) => setSpotifyVolume(level, done)}
      onControl={(action) => {
        if (action === 'playPause') playOrPause()
        else if (action === 'next') skip(1)
        else if (action === 'previous') skip(-1)
      }}
    />
  )
}

/** The web app's site (QQ Music's, NetEase Cloud Music's, Bilibili's or Douyin'sng FM's own page): the song it
 * plays, its cover and the line of its words it's at, opening the app that shows it; its play/pause works on
 * the page itself, and the lyrics button comes with a song's own words (`wordsAreSongs`: Kugou Music's too, as
 * its own player keeps the whole of them). */
function WebAppWidget({ webApp }: { webApp: NonNullable<ReturnType<typeof useWebAppSource>> }) {
  const site = useWebApp((store) => store.site)
  const volume = useWebApp((store) => store.volume)
  // Its words, whole, are the cabin's own view's to draw (LyricsPage.tsx): the button comes with them,
  // and only for the music sites (a video's or a podcast's words aren't a song's).
  const music = wordsAreSongs(site)
  const hasLyrics = useWebApp((store) => (store.lyrics?.lines.length ?? 0) > 0)
  // Bilibili's player has a volume control of its own, and the cabin's slider overrides it: no slider there
  // (webapp.ts `cabinVolumeFor`).
  const volumes = cabinVolumeFor(site)
  return (
    <MusicWidget
      source={webApp.source}
      artwork={webApp.artwork}
      onLyrics={music && hasLyrics ? () => useWebApp.setState({ lyricsOpen: true }) : undefined}
      volume={volumes ? volume : null}
      onVolume={volumes ? (level) => webAppCommand({ action: 'setVolume', volume: level }) : undefined}
      onOpen={() => {
        go({ type: 'openApp', app: site ? APP_OF_SITE[site] : 'youtube' })
      }}
      onControl={(action) => {
        if (action === 'playPause') webAppCommand({ action: 'playPause' })
        if (action === 'next') webAppCommand({ action: 'next' })
        if (action === 'previous') webAppCommand({ action: 'previous' })
      }}
    />
  )
}

/**
 * Behind Now Playing's words, in each look: the blurred cover at its opacity (dashboard.css) over the look's
 * glass, on the page behind an app (what's under the glass, roughly). */
const WIDGET_BACKDROPS: Record<Look, Backdrop> = {
  dark: { cover: 0.55, rest: [40, 40, 42] },
  light: { cover: 0.55, rest: [248, 248, 250] },
}

/**
 * Now Playing's music: a source's track, tinted by its album art (a blurred copy behind the glass). Track
 * changes animate (the new cover waited for, the text sliding, cover and tint crossfading). The track opens
 * its app (if WestPlay has one for it); the controls work in place, on what's playing now. Over a cover the
 * look's words wouldn't read on, the other look's (a light cover: dark words); the glass and tint stay.
 */
function MusicWidget({
  source,
  artwork,
  onOpen,
  onLyrics,
  volume = null,
  onVolume,
  onControl,
}: {
  source: MediaSource | null
  artwork: string | null
  onOpen?(): void
  /** The words it plays, whole, to open (a source that has them: the web app's page). */
  onLyrics?(): void
  /** What it plays at, 0 to 1 (`null`: the player keeps its own, nothing to drag). */
  volume?: number | null
  /** That volume, dragged (`done`: the drag let go: the player's own word can be sent then). */
  onVolume?(level: number, done: boolean): void
  onControl(action: MediaAction): void
}) {
  const transition = useTrackTransition(source, artwork)
  const albumArt = useCabin((state) => state.albumArt)
  const cover = albumArt ? (transition.shown?.artwork ?? null) : null
  const controls = source?.controls
  const playing = source?.status === 'playing'
  const text = useText()
  const ink = useWordsOver(cover, useLook().appearance, WIDGET_BACKDROPS)

  return (
    <article className={`widget glass lg-system now-playing ${wordsClass(ink)}`}>
      <Crossfade value={cover} className="np-tint" render={(url) => url && <img src={url} alt="" aria-hidden />} />
      <button type="button" className="np-top" onClick={onOpen} disabled={!onOpen}>
        <Crossfade
          value={cover}
          className="art"
          render={(url) =>
            url ? (
              <img src={url} alt="" referrerPolicy="no-referrer" />
            ) : (
              <span className="art-placeholder" aria-hidden>
                <NoteGlyph size={36} />
              </span>
            )
          }
        />
        <TrackSlide
          transition={transition}
          className="track"
          layerClassName="track-lines"
          render={(track) => (
            <>
              <span className="title">{track?.source.title ?? text.notPlaying}</span>
              {track && <span className="artist">{track.source.lyric ?? track.source.artist}</span>}
            </>
          )}
        />
      </button>
      <div className="controls">
        {onLyrics && (
          <button type="button" className="widget-lyrics" aria-label={text.appleMusic.lyrics} onClick={onLyrics}>
            <LyricsGlyph size={30} />
          </button>
        )}
        <button
          type="button"
          aria-label={text.previous}
          disabled={!controls?.previous}
          onClick={() => onControl('previous')}
        >
          <PreviousGlyph size={34} />
        </button>
        <button
          type="button"
          aria-label={playing ? text.pause : text.play}
          disabled={!controls?.playPause}
          onClick={() => onControl('playPause')}
        >
          {playing ? <PauseGlyph size={40} /> : <PlayGlyph size={40} />}
        </button>
        <button type="button" aria-label={text.next} disabled={!controls?.next} onClick={() => onControl('next')}>
          <NextGlyph size={34} />
        </button>
      </div>
      {/* What it plays at, under the controls: the player's own volume, not WestPlay's (owner 2026-10-30). */}
      {onVolume && volume !== null && (
        <div className="np-volume">
          {/* A speaker before it: a bar with no words beside it reads as the song's progress. */}
          <span className="np-speaker" aria-hidden>
            <SpeakerGlyph size={20} volume={volume} />
          </span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={volume}
            aria-label={text.appleMusic.volume}
            onChange={(event) => onVolume(Number(event.target.value), false)}
            onPointerUp={() => onVolume(volume, true)}
            onKeyUp={() => onVolume(volume, true)}
          />
        </div>
      )}
    </article>
  )
}

/**
 * Where the truck is headed, our "live activity" (destination.ts): the job's pickup and then its delivery
 * (deadline and distance left), or where the game's route ends; folded away without either, as the
 * guidance card comes and goes.
 */
export function DestinationWidget() {
  const view = useDestinationView()
  const shown = useCabin((state) => state.dashboardWidgets.destination) && view !== EMPTY
  return (
    <div className={`widget-slot${shown ? ' open' : ''}`}>
      {shown && (
        <article className="widget glass lg-system job">
          <div className="kicker">{view.kicker}</div>
          <div className="destination">{view.title}</div>
          {view.subtitle && <div className="place">{view.subtitle}</div>}
          <div className={`deadline${view.late ? ' late' : ''}`}>{view.detail}</div>
        </article>
      )}
    </div>
  )
}

/**
 * The Dashboard's photo widget: a picture of the player's own — one of the wallpaper gallery's, chosen in
 * Settings › Dashboard — cropped into the card. Nothing chosen, it shows the wallpaper on screen, so turning the
 * widget on shows a picture at once (owner 2026-11-03). No name over it: the picture is the card (owner
 * 2026-11-03, later that day); the name is its alt text, for whoever reads the screen.
 */
export function PhotoWidget() {
  const text = useText()
  const shown = useCabin((state) => state.dashboardWidgets.photo)
  const chosen = useCabin((state) => state.dashboardPhoto)
  const wallpaperId = useCabin((state) => state.wallpaperId)
  const wallpapers = useWallpapers()
  const { appearance } = useLook()
  const picture = photoView(chosen === '' ? wallpaperId : chosen, wallpapers, appearance)
  const src = usePhotoBitmap(picture.src)
  return (
    <div className={`widget-slot${shown ? ' open' : ''}`}>
      {shown && (
        <article className="widget glass photo" aria-label={text.settings.photoWidget}>
          <img src={src} alt={picture.name} />
        </article>
      )}
    </div>
  )
}

/** Card-sized bitmaps of the pictures, by source (each once: a wallpaper file can be 4000 px wide). */
const photoBitmaps = new Map<string, Promise<string>>()

/** The picture's room in the column, twice over, so it stays sharp (dashboard.css). */
const PHOTO_WIDTH = 900
const PHOTO_HEIGHT = 560

/** The picture at the card's size, drawn once: a wallpaper's own file is too big for the card to scale per frame. */
function usePhotoBitmap(src: string): string {
  const [bitmap, setBitmap] = useState<string | null>(null)
  useEffect(() => {
    let current = true
    const made = photoBitmaps.get(src) ?? prepareWallpaper(src, PHOTO_WIDTH, PHOTO_HEIGHT).catch(() => src)
    photoBitmaps.set(src, made)
    void made.then((url) => {
      if (current) setBitmap(url)
    })
    return () => {
      current = false
    }
  }, [src])
  return bitmap ?? src
}

/**
 * The Dashboard's Vehicle status widget: the truck's body's damage, and the trailer's while one is hitched — the
 * game's own numbers, without the tyres (owner 2026-11-03: "车辆状态" on top, "车身：x%" under it, and the
 * trailer's with one).
 */
export function VehicleWidget() {
  const shown = useCabin((state) => state.dashboardWidgets.vehicle)
  const labels = useText().vehicleApp
  const body = useLive((state) => bodyDamage(state.telemetry))
  const attached = useLive((state) => hitched(state.telemetry, state.trailer))
  const trailer = useLive((state) => (hitched(state.telemetry, state.trailer) ? trailerDamage(state.telemetry) : null))
  return (
    <div className={`widget-slot${shown ? ' open' : ''}`}>
      {shown && (
        <article className="widget glass vehicle-status" aria-label={labels.condition}>
          <div className="kicker">{labels.condition}</div>
          {body === null ? (
            <div className="vehicle-waiting">{labels.unavailable}</div>
          ) : (
            <>
              <div className="vehicle-line">{labels.body(percent(body))}</div>
              {attached && (
                <div className="vehicle-line trailer">
                  {labels.trailerDamage(trailer === null ? null : percent(trailer))}
                </div>
              )}
            </>
          )}
        </article>
      )}
    </div>
  )
}

/** How many widgets the column holds at most: the weather comes in only where there's room. */
const COLUMN = 3

/**
 * The weather where the truck is, as CarPlay's weather widget (weather.ts): the nearest city, the weather
 * and its symbol. The column holds three widgets: it comes in where there's room (turn-by-turn, Now Playing,
 * the destination, the picture or the truck's status not showing), as they come and go; and not without the
 * weather (no game, or one whose weather isn't read), or turned off itself.
 */
export function WeatherWidget() {
  const text = useText()
  const labels = useMapData()?.labels
  const widgets = useCabin((state) => state.dashboardWidgets)
  const guided = useGuided()
  const destination = useDestinationView()
  // Every other card counts: the picture and the truck's status too, so the weather is the one that yields.
  const others = [
    guided,
    widgets.nowPlaying,
    widgets.destination && destination !== EMPTY,
    widgets.photo,
    widgets.vehicle,
  ].filter(Boolean).length
  const room = widgets.weather && others < COLUMN
  const weather = useLive((state) => state.weather)
  const gameTime = useLive((state) => (state.telemetry ? cabClock(state.telemetry, state.clockShift) : 0))
  const place = useLive((state) => {
    const t = state.telemetry
    return t && labels ? nearestCity([t.x, t.z], labels) : null
  })
  const view = weather ? weatherView(weather, gameTime, place, text) : null
  return (
    <div className={`widget-slot${room && view ? ' open' : ''}`}>
      {view && (
        <article
          className="widget glass lg-system weather"
          aria-label={[view.place, view.condition].filter(Boolean).join(', ')}
        >
          <div className="weather-text">
            {view.place && <div className="place">{view.place}</div>}
            <div className="condition">{view.condition}</div>
          </div>
          <WeatherGlyph symbol={view.symbol} size={84} />
        </article>
      )}
    </div>
  )
}
