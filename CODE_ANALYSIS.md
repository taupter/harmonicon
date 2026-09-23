# Code analysis progress

Started 2026-09-23. Review order: root composition crate, then feature crates, shared Bevy crates, and Bevy-free crates, following `contributing/src/overview.md` and `contributing/src/module-dependency-rules.md`. Preserve public APIs unless a larger architectural benefit is demonstrated. Existing user changes in `Cargo.toml`, `Cargo.lock`, `crates/harmonicon-audio/src/pipeline.rs`, `CHANGELOG.md`, and packaging files are outside this review and must be preserved.

## Completed files

- `src/main.rs` — thin entry point; no change needed.
- `src/lib.rs` — plugin assembly and startup ordering checked against the contributor architecture guide; removed a stale Bevy 0.19 RC comment. No hot loop here.
- `src/dev_capture.rs` — dev-only screen capture; `Image` cloning is required by the consuming conversion API. No change needed.
- `src/bin/gen_synthetic_dataset.rs` — thin I/O wrapper; no change needed.
- `src/bin/note_bench.rs` — sorting recording directories requires materialization; no significant copy or hot-loop issue in this wrapper.
- `src/bin/hole_editor.rs` — skip mesh, material, and status text updates while editor state is unchanged; this avoids repeated per-frame string formatting and asset mutation. No public API.
- `src/bin/note_editor.rs` — avoid marking editor state changed every frame when the resize key is unchanged; format status text only on actual state changes. Existing preview updates already use change detection. No public API.
- `crates/harmonicon-dsp/src/lib.rs` — removed the per-frame harmonic suppression bitmap and the NMF activation loop's 50 temporary denominator vectors per analysis block. Public API unchanged. `cargo test -p harmonicon-dsp` passed (31 tests).
- `contributing/src/overview.md` — corrected the Bevy version, desktop entry-point description, and inaccurate blanket claim about re-export facades.
- `crates/harmonicon-lessons/src/lib.rs` — plugin registration and schedule order match the documented lesson/menu boundary; no change needed.
- `crates/harmonicon-lessons/src/lesson_tree/layout.rs` — removed per-node temporary vectors from barycentre calculation and reused layer storage during ordering sweeps. Layout behavior unchanged; all 76 lessons tests passed.
- `crates/harmonicon-lessons/src/lesson_tree/edges.rs` — matched the documented material-sharing behavior by reusing solid-edge handles per style and diagonal within a tree build, instead of allocating one asset per edge. `lesson_tree/mod.rs` owns the small local cache; its broader review is still pending. All 76 lessons tests passed.
- `crates/harmonicon-lessons/src/lesson_tree/transition.rs` — skip idle slide updates, repeated viewport resource writes, and per-frame chevron string allocation when the label is unchanged. All 76 lessons tests passed.
- `crates/harmonicon-lessons/src/lesson_tree/mod.rs` — removed a cloned unit-ID vector during tree setup and the temporary localized-title vector for locked-node tooltips. Existing ECS-owned IDs and image handles still require cloning. All 76 lessons tests passed.
- `crates/harmonicon-lessons/src/lesson_reader/mod.rs` — page assembly and widget constructors reviewed. Initialize the displayed BPM cache for each metronome and phrase looper. The remaining manifest clones belong to owned Bevy components or button observers. All 76 lessons tests and Clippy passed.
- `crates/harmonicon-lessons/src/lesson_reader/widgets.rs` — eliminated per-frame BPM label formatting and text replacement while the tempo is unchanged. All 76 lessons tests and Clippy passed.
- `crates/harmonicon-menu/src/lib.rs` — replaced malformed and inaccurate crate documentation; retained the existing public re-export API.
- `crates/harmonicon-menu/src/menu/mod.rs` — plugin registration and page cleanup ordering reviewed; no change needed.
- `crates/harmonicon-menu/src/menu/routing.rs` — routing and Escape navigation reviewed; corrected a stale reference to the removed lesson list page.
- `crates/harmonicon-menu/src/menu/scene.rs` — shared page and button helpers reviewed; removed documentation that described an older themed-button implementation and clarified the root marker's actual role.
- `crates/harmonicon-menu/src/menu/pages/artist_list.rs` — artist sorting already uses borrowed names; observer clones an owned artist name only when selecting. No change needed.
- `crates/harmonicon-menu/src/menu/pages/song_list.rs` — sort borrowed song entries instead of cloning the complete song list before spawning buttons. The observer still owns its asset path. All 36 menu tests passed.

## Findings and follow-up

- `contributing/src/plugin-architecture.md` still contains stale Bevy 0.19 and `main.rs` wiring descriptions. A full rewrite should follow the actual current plugin registration; this is not yet marked complete.
- DSP already uses `rustfft`, which can select SIMD implementations for FFTs. YIN/MPM difference loops and NMF dot products may benefit from explicit SIMD or `simsimd`, but require representative recordings and benchmarks before changing floating-point reduction order or adding a dependency. NMF also allocates its output spectrum and activation vectors each call; changing that without affecting the public return type needs a careful buffer-reuse design.
- Next review: remaining `harmonicon-menu/src/menu/pages/` files, then Options and Harp Check.
