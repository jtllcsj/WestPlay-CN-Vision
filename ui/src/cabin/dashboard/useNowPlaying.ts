import { useEffect } from 'react'
import type { MediaSource } from '../../shared/media'
import { command, useYouTube } from '../apps/youtube/youtube'
import { playOrPause, useSpotifySource } from '../apps/spotify/spotify'
import { playOrPause as playOrPauseAppleMusic, useAppleMusicSource } from '../apps/music/appleMusic'
import { playOrPause as playOrPauseYouTubeMusic, useYouTubeMusicSource } from '../apps/youtubeMusic/youtubeMusic'
import { command as webAppCommand, shownPlaying, useWebApp } from '../apps/web/webapp'
import { playOrPause as playOrPausePlayer, usePlayerSource } from '../apps/player/player'
import { playOrPauseRadio, useRadioSource } from '../apps/radio/radio'
import { playOrPause as playOrPauseQqMusic, useQqMusicSource } from '../apps/qq/qqMusic'
import { playOrPause as playOrPauseNetease, useNeteaseSource } from '../apps/netease/netease'
import { type NowPlayingSource, shownSource } from './nowPlaying'

/** What Now Playing last showed playing, kept while the screens showing it come and go (nowPlaying.ts). */
let lastPlayed: NowPlayingSource | null = null

/**
 * What the web app's site plays (QQ Music's, NetEase Cloud Music's, Bilibili's or Douyin'sng FM's own page), as
 * Now Playing shows it: the song the site's Media Session says plays (or its player bar draws), its cover, and
 * the line of its words the page's own lyrics put it at (westplay_host::webapp).
 */
export function useWebAppSource(): { source: MediaSource; artwork: string | null } | null {
  const media = useWebApp((store) => store.media)
  const site = useWebApp((store) => store.site)
  const playing = useWebApp(shownPlaying)
  if (!media || (media.title === '' && media.artist === '')) return null
  return {
    artwork: media.artwork,
    source: {
      id: `westplay-webapp-${site ?? 'page'}`,
      service: 'webApp',
      title: media.title,
      artist: media.artist,
      album: '',
      artwork: media.artwork,
      status: playing ? 'playing' : 'paused',
      // The page doesn't say where in the song it is (the widget draws no progress either way).
      positionMs: 0,
      positionAt: 0,
      durationMs: 0,
      controls: { playPause: true, next: true, previous: true, shuffle: false, repeat: false },
      shuffle: null,
      repeat: null,
      lyric: media.lyric,
    },
  }
}

/**
 * Bilibili's video, as a song: its title, its uploader, its cover.
 *
 * The Dashboard's Now Playing draws a music card for the music apps (cover, title, artist, the
 * three buttons). A video got the video widget instead — its own picture filling the card. Bilibili's
 * has a cover and a title like a song's, so it goes the music card's way, as the music apps do
 * (owner 2026-11-25). YouTube's keeps the picture: its videos are the thing being watched.
 */
export function useBilibiliSource(): { source: MediaSource; artwork: string | null } | null {
  const video = useYouTube((store) => (store.videoSite === 'bilibili' ? store.current : null))
  const held = useYouTube((store) => store.states.bilibili)
  if (!video) return null
  const mine = held.state?.video === video.video ? held.state : null
  const playing = mine?.state === 1
  return {
    artwork: video.thumbnail,
    source: {
      id: `westplay-bilibili-${video.video}`,
      service: 'bilibili',
      title: video.title,
      artist: video.channel,
      album: '',
      artwork: video.thumbnail,
      status: playing ? 'playing' : 'paused',
      // Its player says where the video is, and when it said so (`held.at`): the card's progress.
      positionMs: Math.round((mine?.time ?? 0) * 1000),
      positionAt: mine ? Math.round(performance.timeOrigin + held.at) : 0,
      durationMs: Math.round((mine?.duration ?? 0) * 1000),
      controls: { playPause: true, next: true, previous: true, shuffle: false, repeat: false },
      shuffle: null,
      repeat: null,
    },
  }
}

/** What Now Playing shows (`shownSource`): which player, and each player's own state (the screen showing it
 * draws it its own way: the Dashboard's widget, the Equalizer's track). */
export function useNowPlaying() {
  const webApp = useWebAppSource()
  const spotify = useSpotifySource()
  const appleMusic = useAppleMusicSource()
  const youtubeMusic = useYouTubeMusicSource()
  const player = usePlayerSource()
  const qqMusic = useQqMusicSource()
  const netease = useNeteaseSource()
  const bilibili = useBilibiliSource()
  const radio = useRadioSource()
  const video = useYouTube((store) => store.current)
  // Bilibili's videos go the music card's way (cover and title, the three buttons); YouTube's keep
  // the picture. `bilibili` above is that same video as a song, so the `video` slot only stands
  // for YouTube's now.
  const bilibiliShown = bilibili !== null
  const videoState = useYouTube((store) => store.states.youtube.state)
  const videoPlaying = videoState?.state === 1 && videoState.video === video?.video
  const playing = {
    video: video !== null && videoPlaying && !bilibiliShown,
    bilibili: bilibili?.source.status === 'playing',
    webApp: webApp?.source.status === 'playing',
    spotify: spotify?.source.status === 'playing',
    appleMusic: appleMusic?.source.status === 'playing',
    youtubeMusic: youtubeMusic?.source.status === 'playing',
    qqMusic: qqMusic?.source.status === 'playing',
    netease: netease?.source.status === 'playing',
    player: player?.source.status === 'playing',
    radio: radio?.source.status === 'playing',
  }
  const has = {
    video: video !== null && !bilibiliShown,
    bilibili: bilibiliShown,
    webApp: Boolean(webApp),
    spotify: Boolean(spotify),
    appleMusic: Boolean(appleMusic),
    youtubeMusic: Boolean(youtubeMusic),
    qqMusic: Boolean(qqMusic),
    netease: Boolean(netease),
    player: Boolean(player),
    radio: Boolean(radio),
  }
  const showing = shownSource(playing, has, lastPlayed)
  const showingPlays = showing !== null && playing[showing]
  useEffect(() => {
    if (showing !== null && showingPlays) lastPlayed = showing
  }, [showing, showingPlays])
  return { showing, playing, bilibili, webApp, spotify, appleMusic, youtubeMusic, qqMusic, netease, player, radio, video }
}

/** The track Now Playing shows, as one line of a list draws it (the Equalizer's): its title, who it's by, its
 * cover, whether it plays, and playing or pausing it; `null`: nothing to show. */
export function useNowPlayingTrack(): {
  title: string
  by: string
  artwork: string | null
  playing: boolean
  playPause: (() => void) | null
} | null {
  const { showing, playing, webApp, spotify, appleMusic, youtubeMusic, qqMusic, netease, player, radio, video } =
    useNowPlaying()
  const music = (
    player: { source: { title: string; artist: string; controls: { playPause: boolean } }; artwork: string | null },
    playPause: () => void,
    plays: boolean,
  ) => ({
    title: player.source.title,
    by: player.source.artist,
    artwork: player.artwork,
    playing: plays,
    playPause: player.source.controls.playPause ? playPause : null,
  })
  switch (showing) {
    case 'video':
      return video
        ? {
            title: video.title,
            by: video.channel,
            artwork: video.thumbnail,
            playing: playing.video,
            playPause: () => command({ action: playing.video ? 'pause' : 'play' }),
          }
        : null
    case 'webApp':
      return webApp ? music(webApp, () => webAppCommand({ action: 'playPause' }), playing.webApp) : null
    case 'spotify':
      return spotify ? music(spotify, playOrPause, playing.spotify) : null
    case 'appleMusic':
      return appleMusic ? music(appleMusic, playOrPauseAppleMusic, playing.appleMusic) : null
    case 'youtubeMusic':
      return youtubeMusic ? music(youtubeMusic, playOrPauseYouTubeMusic, playing.youtubeMusic) : null
    case 'qqMusic':
      return qqMusic ? music(qqMusic, playOrPauseQqMusic, playing.qqMusic) : null
    case 'netease':
      // NetEase's app is the player itself (its own element in the cabin page), so this is the app's own
      // pause and play, not a page's.
      return netease ? music(netease, playOrPauseNetease, playing.netease) : null
    case 'player':
      return player ? music(player, playOrPausePlayer, playing.player) : null
    case 'radio':
      return radio ? music(radio, playOrPauseRadio, playing.radio) : null
    default:
      return null
  }
}
