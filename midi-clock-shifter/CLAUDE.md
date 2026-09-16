# CLAUDE.md — Midi Clock Shifter

Windows desktop app (Rust, egui, crate `midi_clock_shifter`) that receives MIDI clock, predicts it, and re-emits it with a phase offset — normally ahead of time, to compensate the video latency between Resolume and the projector — to one or more MIDI outputs. Part of the `vj_stuff` monorepo, developed standalone from this folder.

## Read these first, in this order

1. **`ARCHITECTURE.md`** — the authoritative design: signal flow, virtual ports, clock prediction, scheduler, start/stop/resync, UI layout, dependencies, quality bar.
2. **`PROMPTS.md`** — the step-by-step build plan. Execute exactly one prompt at a time and do not start the next one until the user confirms the current one works.
3. **The project skills** in `.claude/skills/` (use the Skill tool, or read the `SKILL.md` files directly if they aren't listed as available):
   - **midi-clock-protocol** — protocol reference: status bytes, PPQN math, and what "shift" means in this app (phase offset via prediction, re-timed Start). Read before touching clock parsing, generation, or shift logic.
   - **timing-and-jitter** — rules for low-jitter, drift-free timing on the hot path (callbacks and scheduler). Read before touching scheduling, sleeps, channels, or anything between receiving a byte and sending one.
   - **rust-project-conventions** — module layout, UI requirements, dependency choices, error handling, and style. Read before adding a module, a dependency, or deciding where code belongs.

### If documents disagree

`ARCHITECTURE.md` wins over the skills, and the skills win over general habits. If something in `ARCHITECTURE.md` is technically wrong or unclear (e.g. an FFI signature), **stop, explain the problem and propose a correction** before writing code. When the design changes, update `ARCHITECTURE.md` (and the affected skill) in the same change so they stay consistent.

## Ground rules

- Timing correctness is the whole point of this app — never take a shortcut on the hot path (allocation, blocking locks, logging) without checking **timing-and-jitter** first.
- Only phase offset is implemented (−4000 … +4000 ms, positive = earlier). No tempo scaling.
- Keep `engine/` and `clock.rs` independent of any concrete MIDI crate so they stay unit-testable with the simulated time source; never assert real wall-clock timing in `cargo test`.
- Windows 10/11 x64 is the only target. Virtual ports require the virtualMIDI driver (installed with loopMIDI or the virtualMIDI SDK). Never guess FFI signatures or flag values — take them from `teVirtualMIDI.h`, and ask the user for the header if it isn't in the repo. Do not commit the SDK DLL.
- `cargo fmt`, `cargo build --release` and `cargo clippy --all-targets -- -D warnings` must be clean, and the acceptance checks of the current prompt must pass, before calling a change done.

## Git

This app lives inside the `vj_stuff` monorepo at `midi-clock-shifter/`. Follow the monorepo's git conventions (branch prefix `claude/`, English commit messages) from the repo-root `CLAUDE.md` unless told otherwise.
