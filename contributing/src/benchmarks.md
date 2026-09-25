# Criterion benchmarks

Tracy shows where one real session spends its time. Criterion answers a
narrower question repeatably: how long one function takes on a fixed
input, and whether a change made it faster. Use it before and after
optimizing anything on a live path.

```bash
cargo bench -p harmonicon-bench --bench detectors
cargo bench -p harmonicon-gameplay --bench judge -- score_frame   # filter by name
```

Run benches without `dev`: that is the build players get, and `dev`
compiles in extra systems and Bevy's debug features. Reports land in
`target/criterion/`, with HTML under `report/index.html`.

## What exists

Each bench's `//!` header says what it measures and why.

| Bench | Measures | Runs |
|---|---|---|
| `harmonicon-bench` `detectors` | `pitch_detect::analyze` per algorithm, warm and fresh `FftState` | every audio hop |
| `harmonicon-bench` `core_hot_paths` | `HarmonicaNoteTracker::update`, `tick_to_seconds`/`seconds_to_tick`, `synth::render_pcm` | every hop / every frame / on demand |
| `harmonicon-gameplay` `judge` | `score_notes`, `build_scheduled_notes` | every frame / on a note-list rebuild |
| `harmonicon-gameplay` `note_tails` | `animate_note_tails` with the asset plugin | every frame |
| `harmonicon-jam` `backing` | backing stems, band answer, ending hit | Start / mid-session / session end |

`CODE_ANALYSIS.md` records the baseline numbers and what each one means for
the frame budget.

## Where a new bench goes

- **Bevy-free code goes in `harmonicon-bench/benches/`**, even when the
  function lives in `harmonicon-core` or `harmonicon-dsp`. A dev-dependency
  is compiled for `cargo test` too, and Criterion would slow down the
  core crates' fast test loop.
- **ECS code goes in the owning crate's `benches/`**, running the real
  system in a minimal `World` or `App` the way the crate's tests do. A
  bench can only reach public items, so a system it measures has to be
  `pub`.
- Each bench needs a `[[bench]]` entry with `harness = false`.

## Comparing a change

```bash
cargo bench -p <crate> --bench <name> -- --save-baseline before
# make the change
cargo bench -p <crate> --bench <name> -- --baseline before
```

Keep inputs identical across iterations. If the routine mutates state,
rebuild the state in `iter_batched`'s setup closure, and return anything
large from the routine so Criterion drops it outside the timed section.
Otherwise the timing measures the drop of a song-sized `Vec` instead of
the function.

## Limits

- **Main-world CPU only.** A modified material's re-extraction and GPU
  upload happen in the render world, which needs a device. Capture a
  session in [Tracy](profiling.md) for that half.
- **Not timed in CI.** Shared runners are too noisy for results to mean
  anything. `cargo clippy --all-targets` already compiles every bench, so
  a bench cannot silently stop building.
- Numbers are specific to the machine they ran on. Compare baselines from
  the same machine, not figures copied from `CODE_ANALYSIS.md`.
