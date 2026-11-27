/**
 * NetEase Cloud Music's sound, placed in the truck's cab like the music services' is.
 *
 * The host places every music page's sound through the truck's own speakers (`cab_sound.js`, a graph in the
 * browser's own 3D sound). NetEase Cloud Music is not a page the host runs — it is the cab page's own app, playing
 * through its own `Audio` element — so until now it played as plain stereo while the other players came from the
 * truck's speakers. The host now sends the same cab it sends the music pages to this page (`{"type":"cab", …}`:
 * the speakers, the driver's head, the camera and the tone), and this module takes the app's element into the same
 * graph, so its sound comes from the speakers too (owner 2026-11-20).
 *
 * The graph is `cab_sound.js`'s, kept here in TypeScript and to the same numbers: the element's sound is split to
 * each speaker's band (Linkwitz-Riley pairs), placed at the speaker with a `PannerNode` in HRTF around the driver's
 * head, and mixed for the distance so it is as loud as the sound was at the seat. Without speakers (spatial off, or
 * a truck none are known for) the sound plays as it is, through the tone and the cab's walls alone.
 */

import type { CabHead, Equalizer, Speaker } from '../../../shared/host'

/**
 * Whether the app's sound goes through the cab's graph (`wire`) or plays as the element gives it.
 *
 * Off for now: in WebView2 an element taken in by `createMediaElementSource` never reaches the speakers —
 * not even wired straight to the destination, with the context `running` and the element `paused: false`,
 * `readyState: 4`, `volume: 0.47` (checked 2026-11-20, by bisection: the plain element plays, the same
 * element through the graph is silent — and again 2026-11-25 on WebView2 158: switching this off left the
 * app with no sound at all, so it still holds). The graph itself is kept whole: everything it needs is here, and
 * the switch is the one line below. What it needs next is a way to reach the speakers in this browser —
 * a hidden page of its own, as the other players have, which is the shape `cab_sound.js` is written for.
 */
const BYPASS = true

/** A speaker's sound falls with its distance from the head past this, as in a cab (metres). */
const NEAR = 0.3

/** The camera outside the cab: level (dB) and lowpass (Hz) by distance, gliding between rows. */
const MUFFLE: [number, number, number][] = [
  [4, -6, 250],
  [5, -9, 224],
  [6, -12, 204],
  [8, -16, 177],
  [16, -24, 125],
]
const SILENT = 20

/** The tone's three bands, as a car stereo's. */
const TONE: [keyof Equalizer, BiquadFilterOptions][] = [
  ['bass', { type: 'lowshelf', frequency: 100 }],
  ['mid', { type: 'peaking', frequency: 1000, Q: 0.7 }],
  ['treble', { type: 'highshelf', frequency: 8000 }],
]

/** A head tracker never holds still; the sound moves only past these (a degree, 2 cm), to avoid a faint crackle. */
const STILL_TURN = 1 / 360
const STILL_MOVE = 0.02

/** The cab the host last told this page. */
let speakers: Speaker[] | null = null
let head: CabHead | null = null
let outside = false
let distance = 0
let equalizer: Equalizer = { bass: 0, mid: 0, treble: 0 }

/** The element whose sound the graph takes, once `setSource` names it. */
let sourceElement: HTMLAudioElement | null = null

/** The graph itself, made on first need. */
interface Graph {
  context: AudioContext
  /** The element taken in (an element can be taken in once; a new one gets a new source). */
  source: MediaElementAudioSourceNode | null
  /** Every node after the source, kept so a rewire can drop them. */
  nodes: AudioNode[]
  /** The tone's bands and its trim, before the split. */
  tone: { bands: BiquadFilterNode[]; trim: GainNode } | null
  /** The walls' lowpass and gain, after the limiter (or right after the tone with no speakers). */
  muffle: { filter: BiquadFilterNode; gain: GainNode } | null
  /** The limiter, before the walls (kept to read its reduction if ever needed). */
  limiter: DynamicsCompressorNode | null
  /** The speakers the graph is wired for, so a change rewires it. */
  wired: Speaker[] | null
}

let audio: Graph | null = null
/** An element can be taken into a context once; keep each one's source by its element. */
const sources = new WeakMap<HTMLAudioElement, MediaElementAudioSourceNode>()

/** The head as last placed in the listener, for the hold-still rule. */
let placed: CabHead | null = null

/** The cab the host told this page: `receiveCab` is the page's one way in. */
export function receiveCab(message: {
  speakers: Speaker[] | null
  head: CabHead
  outside: boolean
  distance: number
  equalizer: Equalizer
}): void {
  speakers = message.speakers
  head = message.head
  outside = message.outside
  distance = message.distance
  equalizer = message.equalizer
  try {
    wire()
  } catch (error) {
  }
}

/**
 * The element the sound comes from: the app's own `Audio`. It's taken into the graph when both it and a cab are
 * there, and left alone otherwise (so a page without a cab plays as it always did).
 */
export function setSource(element: HTMLAudioElement | null): void {
  sourceElement = element
  // BYPASS: `createMediaElementSource` takes the element's own output away for good — a graph it's put into
  // is then the only way its sound can reach the speakers. Not called at all while this is on.
  if (BYPASS) return
  try {
    wire()
    wake()
  } catch (error) {
  }
}

/** The sound through the cab, made now if the page can, and rewired as the speakers change. */
/**
 * Wakes the sound graph. A context made outside a user gesture starts `suspended` by the browser's
 * autoplay rule: the element plays, its time runs, and nothing is heard (`currentTime` not moving
 * either — the app showed 0:00). Called as the graph is made and again whenever a song starts, since
 * `resume()` only takes effect after one gesture (the player pressing play).
 */
/** Wakes the sound graph; see its own note. */
export function wake(): void {
  if (audio?.context.state === 'suspended') void audio.context.resume().catch(() => {})
}

/**
 * Whether two speaker lists describe the same cab. The host sends an array with every message, and the graph
 * only needs rebuilding when what it says changes.
 */
function sameSpeakers(a: Speaker[], b: Speaker[] | null): boolean {
  if (!b || a.length !== b.length) return false
  return a.every((speaker, i) => {
    const other = b[i]
    return (
      speaker.channel === other.channel &&
      speaker.level === other.level &&
      speaker.delay === other.delay &&
      speaker.band[0] === other.band[0] &&
      speaker.band[1] === other.band[1] &&
      speaker.at[0] === other.at[0] &&
      speaker.at[1] === other.at[1] &&
      speaker.at[2] === other.at[2]
    )
  })
}


function wire(): void {
  // BYPASS (2026-11-20): the graph is built and every number reads right, yet nothing is heard. Taken out
  // until it's found, so the app plays through its element as before. Set to `false` to bring it back.
  if (BYPASS) return
  const media = sourceElement
  // The sound is the element's own until there are speakers (and a head) to play it, or walls to come through.
  if (!media || (!audio && !(speakers && head) && !outside)) return
  if (!audio) {
    audio = { context: new AudioContext(), source: null, nodes: [], tone: null, muffle: null, limiter: null, wired: null }
  }
  const graph = audio
  if (graph.source?.mediaElement !== media) {
    let source = sources.get(media)
    if (!source) {
      source = graph.context.createMediaElementSource(media)
      sources.set(media, source)
    }
    graph.source = source
    graph.wired = null
    // Made now: a context created without a gesture is suspended, and the sound never comes out.
    wake()
  }
  if (!graph.source) return
  // Already wired for these speakers (and a head is there): nothing to do. Compared by what they are,
  // not by the array's identity: the host sends a fresh array with each message, so the old `!==` test
  // rewired the whole graph again and again — cutting the source off each time, which is heard as no
  // sound at all (owner 2026-11-20).
  if ((graph.wired && sameSpeakers(graph.wired, speakers)) || (speakers && !head)) return

  graph.source.disconnect()
  for (const node of graph.nodes) node.disconnect()
  graph.nodes = []
  graph.tone = null
  graph.muffle = null
  graph.limiter = null
  graph.wired = speakers

  const { context } = graph
  const keep = <T extends AudioNode>(node: T): T => (graph.nodes.push(node), node)


  // First, the tone.
  const bands = TONE.map(([, options]) => keep(new BiquadFilterNode(context, options)))
  const trim = keep(new GainNode(context))
  bands.reduce((from, to) => (from.connect(to), to)).connect(trim)
  graph.source.connect(bands[0])
  graph.tone = { bands, trim }
  setTone(true)

  // The walls, after everything, muffling the sound when the camera is outside.
  const walls = keep(new BiquadFilterNode(context, { type: 'lowpass', frequency: outside ? muffled().frequency : 20000, Q: Math.SQRT1_2 }))
  const through = keep(new GainNode(context, { gain: outside ? muffled().gain : 1 }))
  walls.connect(through)
  graph.muffle = { filter: walls, gain: through }

  if (!speakers || !head) {
    // No speakers: the sound as it is, through the tone and the walls.
    trim.connect(walls)
    through.connect(context.destination)
    placeHead()
    return
  }
  // The cab, held still for the rest of this wire: the module-level values may change while the graph is built.
  const h = head
  const sp = speakers

  // Stereo always, then each side, and both mixed.
  const stereo = keep(new GainNode(context, { channelCount: 2, channelCountMode: 'explicit', channelInterpretation: 'speakers' }))
  const split = keep(context.createChannelSplitter(2))
  trim.connect(stereo)
  stereo.connect(split)
  const channels = {
    left: keep(context.createGain()),
    right: keep(context.createGain()),
    both: keep(new GainNode(context, { gain: 0.5 })),
  }
  split.connect(channels.left, 0)
  split.connect(channels.right, 1)
  split.connect(channels.both, 0)
  split.connect(channels.both, 1)

  // Made up for the distance as the speakers' power adds up at the seat.
  const fall = (at: number[]): number => NEAR / Math.max(NEAR, Math.hypot(at[0] - h.x, at[1] - h.y, at[2] - h.z))
  const seat = Math.sqrt(sp.reduce((sum, speaker) => sum + (fall(speaker.at) * 10 ** ((speaker.level ?? 0) / 20)) ** 2, 0))
  const HEADROOM = 10 ** (-2 / 20)
  const out = keep(new GainNode(context, { gain: Math.min(6, Math.max(1, HEADROOM / seat)) }))
  const limiter = keep(new DynamicsCompressorNode(context, { threshold: -3, knee: 0, ratio: 20, attack: 0.003, release: 0.1 }))
  graph.limiter = limiter
  out.connect(limiter)
  limiter.connect(walls)
  through.connect(context.destination)

  for (const speaker of sp) {
    let node: AudioNode = channels[speaker.channel]
    const [from, to] = speaker.band
    const filters: [BiquadFilterType, number][] = [
      ...(from > 0 ? ([['highpass', from]] as [BiquadFilterType, number][]) : []),
      ...(to > 0 ? ([['lowpass', to]] as [BiquadFilterType, number][]) : []),
    ]
    for (const [type, frequency] of filters) {
      for (let i = 0; i < 2; i++) {
        const filter = keep(new BiquadFilterNode(context, { type, frequency, Q: Math.SQRT1_2 }))
        node.connect(filter)
        node = filter
      }
    }
    const level = keep(new GainNode(context, { gain: 10 ** ((speaker.level ?? 0) / 20) }))
    node.connect(level)
    node = level
    if (speaker.delay > 0) {
      const late = keep(new DelayNode(context, { delayTime: Math.min(speaker.delay, 50) / 1000, maxDelayTime: 0.05 }))
      node.connect(late)
      node = late
    }
    const [x, y, z] = speaker.at
    const panner = keep(
      new PannerNode(context, {
        panningModel: 'HRTF',
        distanceModel: 'inverse',
        refDistance: NEAR,
        rolloffFactor: 1,
        positionX: x,
        positionY: y,
        positionZ: z,
      }),
    )
    node.connect(panner)
    panner.connect(out)
  }


  // The head placed anew (its listener perhaps new too).
  placed = null
  placeHead()
}

/** The tone's bands and trim, set at once or gliding (no click). */
function setTone(atOnce: boolean): void {
  const graph = audio
  if (!graph?.tone) return
  const { currentTime } = graph.context
  const set = (param: AudioParam, value: number): AudioParam =>
    atOnce ? param.setValueAtTime(value, currentTime) : param.setTargetAtTime(value, currentTime, 0.05)
  TONE.forEach(([band], i) => set(graph.tone!.bands[i].gain, equalizer[band]))
  const lift = Math.max(0, ...TONE.map(([band]) => equalizer[band]))
  set(graph.tone!.trim.gain, 10 ** (-lift / 20))
}

/** The camera's walls: how much they muffle, at this distance. */
function muffled(): { frequency: number; gain: number } {
  const last = MUFFLE[MUFFLE.length - 1]
  const d = Math.min(Math.max(MUFFLE[0][0], distance), last[0])
  const after = Math.max(1, MUFFLE.findIndex(([at]) => at >= d))
  const [from, to] = [MUFFLE[after - 1], MUFFLE[after]]
  const t = Math.log2(d / from[0]) / Math.log2(to[0] / from[0])
  const level = from[1] + (to[1] - from[1]) * t
  const frequency = from[2] * (to[2] / from[2]) ** t
  const fade = Math.min(1, Math.max(0, (SILENT - distance) / (SILENT - last[0])))
  return { frequency, gain: 10 ** (level / 20) * fade }
}

/** The head in the listener, moving only past the hold-still rule. */
function placeHead(): void {
  if (!audio || !head) return
  const turn = (a: number, b: number): number => Math.abs(((a - b + 1.5) % 1) - 0.5)
  if (
    placed &&
    turn(head.heading, placed.heading) < STILL_TURN &&
    Math.abs(head.pitch - placed.pitch) < STILL_TURN &&
    Math.hypot(head.x - placed.x, head.y - placed.y, head.z - placed.z) < STILL_MOVE
  )
    return
  placed = { ...head }
  const { listener, currentTime } = audio.context
  const across = head.heading * 2 * Math.PI
  const up = head.pitch * 2 * Math.PI
  const values = {
    positionX: head.x,
    positionY: head.y,
    positionZ: head.z,
    forwardX: -Math.sin(across) * Math.cos(up),
    forwardY: Math.sin(up),
    forwardZ: -Math.cos(across) * Math.cos(up),
    upX: Math.sin(across) * Math.sin(up),
    upY: Math.cos(up),
    upZ: Math.cos(across) * Math.sin(up),
  }
  for (const [name, value] of Object.entries(values)) {
    listener[name as 'positionX'].setTargetAtTime(value, currentTime, 0.03)
  }
}

/** Whether the sound is being routed through the cab (for tests and logs). */
export function throughCab(): boolean {
  return audio?.wired != null
}
