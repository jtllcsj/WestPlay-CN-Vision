import type { YouTubeState } from '../../../shared/host'
import { type MusicPlayer, pauseMusicWithGame } from '../musicWithGame'
import { command, nextVideo, play, positionNow, previousVideo, setVolume, useYouTube } from './youtube'

/** A video playing, or about to (buffering). */
function isPlaying(state: YouTubeState | null): boolean {
  return state !== null && (state.state === 1 || state.state === 3)
}

/** YouTube as one of the cab's players (oneAtATime.ts). */
export const youtubePlayer: MusicPlayer = {
  name: 'youtube',
  // Every command names `youtube` rather than leaving it to `command()`'s default: that default is
  // whichever site the app is showing, and Bilibili is the other one. With the default, Bilibili
  // starting a video made `oneAtATime` pause YouTube — and the pause went to Bilibili's own player,
  // which had just started (`owner 2026-11-25`: "B站播放后会自动暂停").
  playingHere: () => isPlaying(useYouTube.getState().states.youtube.state),
  pauseHere: () => command({ action: 'pause' }, 'youtube'),
  resumeHere: () => command({ action: 'play' }, 'youtube'),
  // The volume every video plays at (none while muted).
  volume: () => {
    const { volume, muted } = useYouTube.getState()
    return muted ? 0 : volume / 100
  },
  sendVolume: (share) => command({ action: 'volume', percent: Math.round(share * 100) }, 'youtube'),
  setVolume: (share) => setVolume(share * 100, 'youtube'),
  // As its Now Playing's buttons (VideoWidget.tsx): the next video, or back to this one's start (past its first
  // five seconds, or with none before) or the one before; where it shows, as it is.
  skip: (direction) => {
    if (direction === 1) {
      const next = nextVideo()
      if (next) play(next, false)
      return
    }
    const before = previousVideo()
    if (positionNow() > 5 || !before) command({ action: 'seek', seconds: 0 }, 'youtube')
    else play(before, false)
  },
  subscribe: (changed) =>
    useYouTube.subscribe((youtube, before) =>
      // Its own state (`states.youtube`), not `state` — that one becomes Bilibili's the moment the
      // app is showing Bilibili, and YouTube starting a video would go unnoticed.
      changed(isPlaying(youtube.states.youtube.state) && !isPlaying(before.states.youtube.state)),
    ),
}

/** YouTube with the game (musicWithGame.ts): a video still starting as the truck's electrics go off is paused
 * as it starts (buffering). Returns the function that stops it. */
export function pauseYouTubeWithGame(): () => void {
  return pauseMusicWithGame(youtubePlayer)
}
