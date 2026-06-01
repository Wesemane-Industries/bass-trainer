# bass-trainer

A terminal-based practice tool for bass guitar. Plug in your bass, pick a key, and the app will quiz you on notes — listening through your audio interface and detecting what you play in real time.

## How it works

1. **Pick your audio device** — select the input where your bass is connected
2. **Pick a channel** — choose the channel on your interface
3. **Pick a key** — chromatic, or any major/minor key
4. **Drill** — the app shows a note on the bass clef; play and hold it until the pitch detector confirms it, then the next note appears

The tab position is revealed only after the note is confirmed, so you have to find it on the neck yourself first.

## Screenshots

### Setup flow

```
┌ Audio input device (enter=select, esc=quit) ────────────────-─┐
│                                                               │
│ > Focusrite USB MIDI  (2 channels)                            │
│   Built-in Microphone  (1 channels)                           │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

```
┌ Key  —  Focusrite USB MIDI  ch0  (enter=select, esc=back) ──-─┐
│                                                               │
│   All notes (chromatic)   (37 notes in range)                 │
│ > C major   (22 notes in range)                               │
│   C minor   (22 notes in range)                               │
│   C# major  (22 notes in range)                               │
│   ...                                                         │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### Drill screen

```
┌──────────────────────────────────────────────────────────────────┐
│ BASS TRAINER  key: G major   device: Focusrite USB MIDI  ch0     │
│ play the note shown on the staff  (hits 2/4)                     │
└──────────────────────────────────────────────────────────────────┘
┌ Bass clef ───────────────────────────────────────────────────────┐
│                                                                  │
│  𝄢                                                               │
│                    ───────────────────────────────               │
│                                                                  │
│                    ───────────────────────────────               │
│                                                                  │
│                    ──────────────── ●  ───────────         D2    │
│                                                                  │
│                    ───────────────────────────────               │
│                                                                  │
│                    ───────────────────────────────               │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
┌ Tab ─────────────────────────────────────────────────────────────┐
│ G|------------------------------------------|                    │
│ D|------------------------------------------|                    │
│ A|------------------------------------------|                    │
│ E|------------------------------------------|                    │
└──────────────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────────────┐
│ listening…   esc/q to quit                                       │
└──────────────────────────────────────────────────────────────────┘
```

Once the pitch detector has confirmed the note (4 consecutive matching frames), the tab is briefly revealed before moving on to the next note:

```
┌──────────────────────────────────────────────────────────────────┐
│ BASS TRAINER  key: G major   device: Focusrite USB MIDI  ch0     │
│ ✓ correct — D2                                                   │
└──────────────────────────────────────────────────────────────────┘
┌ Bass clef ───────────────────────────────────────────────────────┐
│  ...                                                             │
└──────────────────────────────────────────────────────────────────┘
┌ Tab — D2  (3 positions) ─────────────────────────────────────────┐
│ G|------------------------------------------|                    │
│ D|-0----------------------------------------|                    │
│ A|-----------5------------------------------|                    │
│ E|--------------------10--------------------|                    │
└──────────────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────────────┐
│ heard: D2 (+0.4 cents @ 73.4 Hz, clarity 0.94)   esc/q to quit   │
└──────────────────────────────────────────────────────────────────┘
```

## Build & run

```sh
cargo build --release
./target/release/bass-trainer
```

Requires a working audio input. On Linux, ALSA or PipeWire with ALSA compatibility is expected by [cpal](https://github.com/RustAudio/cpal).

## Keybindings

| Key         | Action            |
| ----------- | ----------------- |
| `↑` / `k`   | move cursor up    |
| `↓` / `j`   | move cursor down  |
| `Enter`     | confirm selection |
| `Esc`       | go back           |
| `q` / `Esc` | quit (from drill) |
| `Ctrl-C`    | quit anywhere     |

## Configuration

Settings (last device, channel, and key) are saved automatically to the platform config directory and restored on the next launch.

| Platform | Path                                                                |
| -------- | ------------------------------------------------------------------- |
| Linux    | `~/.config/bass-trainer/config.json`                                |
| macOS    | `~/Library/Application Support/dev.nwesem.bass-trainer/config.json` |

## Note range

The drill pool covers the full playable range of a standard 4-string bass with up to 20 frets:

```
E1 ──── A1 ──── D2 ──── G2
E  A  D  G  C  F  A# D# G# C# F# B  E  A  D  G  C  F  A# D#  (frets 0–20 on each string)
```

When a key is selected, only notes belonging to that scale are included in the pool.

## Pitch detection

Pitch is detected using the [YIN algorithm](http://audition.ens.fr/adc/pdf/2002_JASA_YIN.pdf) with a 4096-sample window and a 2048-sample hop. At 48 kHz (the preferred sample rate) one detection frame is ~43 ms; at 44.1 kHz it is ~46 ms.

A note is confirmed after 4 consecutive frames agree on the same pitch, which takes roughly **170–185 ms** of sustained sound. Playing a wrong note in between resets the counter.

## License

MIT
