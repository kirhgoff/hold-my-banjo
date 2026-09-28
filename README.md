# hold-my-banjo

A chorus simulator for the eastern banjo frog (pobblebonk, *Limnodynastes dumerilii*). It places a number of frogs in a 2D scene, simulates when each one calls, and mixes the "bonk" calls into stereo audio. Each frog is panned and attenuated according to where it sits in the scene.

The original recording of the banjo frogs sound: [audio/banjo-frogs-long.m4a](audio/banjo-frogs-long.m4a).

## Usage

```sh
cargo run --release -- play                    # play the chorus live on the default output device
cargo run --release -- render                  # render to out/chorus.wav and log calls to out/events.csv
cargo run --release -- analyze out/events.csv  # per-frog call intervals and chorus overlap
```

Scenarios are TOML files, and `scenarios/default.toml` is the default. You can override most parameters from the command line (`--frogs`, `--seed`, `--duration`, `--mode`, `--strength`, `--mean-interval`, `--hearing-radius`, …). Run `cargo run -- play --help` to see the full list.

## Call clips

Call samples are loaded from the `.wav` files in `assets/` (configure this with `audio.asset_dir` or `--assets`). If that folder has no WAV files, the simulator uses a synthetic placeholder bonk.

## Behaviour modes

- `independent`: each frog calls on its own jittered rhythm.
- `event_delay`, `event_advance` and `phase_coupled`: frogs react to calls from neighbours within hearing range. These modes are **hypothetical**. They are borrowed from Japanese tree frog models and have not been validated for banjo frogs.

The bundled default scenario is marked `experimental`. Its values are placeholders, not measured species data.

## License

[MIT](LICENSE)
