import type { YouTubeState } from '../../../shared/host'
import { type MusicPlayer, pauseMusicWithGame } from '../musicWithGame'
import { command, nextVideo, play, previousVideo, useYouTube } from './youtube'

/**
 * Bilibili as one of the cab's players.
 *
 * It is YouTube's shell over another site (`youtube.rs` / `bilibili.rs` are two clients), so its
 * controls are the same commands sent to `bilibili` — but it is a player of its own: the volume
 * keys, the "one at a time" rule and the game's pausing all keep a list of the cab's players, and
 * Bilibili has to be in it or a video keeps playing while the keys act on something else
 * (`owner 2026-11-25`: "B站到后台 westplay 的音量控制快捷键就不起效了").
 *
 * Its state is read from `states.bilibili`, not `state`: the latter is whichever site the app is
 * showing, and the keys have to work while the app is closed (a video in the background).
 */
function isPlaying(state: YouTubeState | null): boolean {
  return state !== null && (state.state === 1 || state.state === 3)
}

/** What Bilibili's player last said, wherever the app is showing. */
function bilibiliState(): YouTubeState | null {
  return useYouTube.getState().states.bilibili.state
}

/** Whether the video playing in the background is Bilibili's. */
function showingBilibili(): boolean {
  const { current, videoSite } = useYouTube.getState()
  return current !== null && videoSite === 'bilibili'
}

export const bilibiliPlayer: MusicPlayer = {
  name: 'bilibili',
  playingHere: () => isPlaying(bilibiliState()),
  pauseHere: () => command({ action: 'pause' }, 'bilibili'),
  resumeHere: () => command({ action: 'play' }, 'bilibili'),
  // The volume every video plays at (none while muted). Bilibili's player keeps its own, so this is
  // the app's setting, sent to its page.
  volume: () => {
    const { volume, muted } = useYouTube.getState()
    return muted ? 0 : volume / 100
  },
  sendVolume: (share) => command({ action: 'volume', percent: Math.round(share * 100) }, 'bilibili'),
  setVolume: (share) => {
    const volume = Math.round(Math.min(100, Math.max(0, share * 100)))
    useYouTube.setState({ volume, muted: volume === 0 })
    command({ action: 'volume', percent: volume }, 'bilibili')
  },
  // As its card's buttons (BilibiliWidget): the next video, or back to this one's start (past its
  // first five seconds, or with none before) or the one before.
  skip: (direction) => {
    if (direction === 1) {
      const next = nextVideo()
      if (next) play(next, false)
      return
    }
    const state = bilibiliState()
    const before = previousVideo()
    if ((state?.time ?? 0) > 5 || !before) command({ action: 'seek', seconds: 0 }, 'bilibili')
    else play(before, false)
  },
  subscribe: (changed) =>
    useYouTube.subscribe((youtube, older) => {
      // Only Bilibili's own starting counts, and only when its video is the one on screen (a page
      // that loads reports playing too, for a video the app never opened).
      if (!showingBilibili()) return
      changed(isPlaying(youtube.states.bilibili.state) && !isPlaying(older.states.bilibili.state))
    }),
}

/** Bilibili with the game (musicWithGame.ts): a video still starting as the truck's electrics go
 * off is paused as it starts (buffering). Returns the function that stops it. */
export function pauseBilibiliWithGame(): () => void {
  return pauseMusicWithGame(bilibiliPlayer)
}
