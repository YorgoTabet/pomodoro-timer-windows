# Windows port plan

Port the macOS Pomodoro app (Swift, SwiftUI) at
`/Users/yorgotabet/Projects/Personal/pomodoro timer` to Windows with Tauri 2,
Rust and Svelte 5 + TypeScript. This repo: `/Users/yorgotabet/Projects/Personal/pomodoro-timer-windows`.

The Swift source is the spec. When a task says "port X.swift", read that file
and match its behaviour, defaults, clamps and wording. Where Windows has no
equivalent, the task says what to do instead.

## Decisions already made (by Yorgo)

- Stack: Tauri 2, Rust backend, Svelte 5 + TypeScript frontend (SvelteKit static, already scaffolded).
- The timer lives in Rust. One timer, owned by the backend. Windows only display it and send commands.
- Everything from the Mac app ships, except the desktop widget and RigStudio.
- Music: Windows SMTC (`GlobalSystemMediaTransportControlsSessionManager`), any player.
  Same rule as the Mac: only resume a player we paused ourselves.
- Characters: all six (samurai, ninja, rabbit, general, Power, Boa Hancock), drawn as SVG
  in the web view from the same path data the Swift art files hold.

## Global constraints (bind every task)

1. **Must build and test on macOS.** This Mac cannot run Windows. All code compiles on
   macOS; Windows-only code (SMTC, anything using the `windows` crate) sits behind
   `#[cfg(windows)]` with a no-op fallback on other targets.
2. Gates every task runs before committing:
   - `cd src-tauri && cargo test && cargo clippy -- -D warnings`
   - `npm run check` (svelte-check) and `npx vitest run` once vitest exists.
   Load Rust first with `source "$HOME/.cargo/env"`.
3. Contract between Rust and the web view:
   - Event `timer-state`, emitted on every change and once per second while running, payload
     `TimerState { phase: "focus"|"shortBreak"|"longBreak", isRunning: bool, remainingSeconds: i64,
     totalSeconds: i64, completedFocusSessions: u32 }` (serde `rename_all = "camelCase"`).
   - Event `phase-elapsed`, payload `{ from: Phase, to: Phase }`, emitted when a phase ends or is skipped.
     Characters use it.
   - Event `settings-changed`, payload the full `Settings`.
   - Commands: `get_state`, `start`, `pause`, `toggle`, `skip`, `reset`, `get_settings`,
     `save_settings(settings)`, `get_stats`.
4. Settings are one JSON file, `settings.json`, in the Tauri app config dir. Same keys, defaults
   and clamps as `PomodoroSettings.swift` (camelCase keys). Unknown or missing keys fall back to defaults.
5. Commit messages: written as Yorgo, first person, plain, no em-dashes, imperative subject.
   **No `Co-Authored-By` trailer, no Claude or Anthropic mention anywhere.**
6. Do not add features the Swift app does not have. No extra docs files.
7. Code comments match the Swift files' style: short, explain why, not what.

## Tasks

### Task 1: Rust core rules + CI

Files: `src-tauri/src/core/{mod,phase,settings,engine}.rs`, `.github/workflows/build.yml`,
`src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`.

- Port `Sources/PomodoroCore/Phase.swift`, `PomodoroEngine.swift`, `PomodoroSettings.swift`
  (data, defaults, clamps, load/save JSON; no observation machinery).
- Character choice is a string enum matching `Character.swift` raw values.
- Unit tests for: long break every n pomodoros, durations, auto-start rules, every clamp,
  defaults when the file is missing or broken.
- CI: GitHub Actions on `windows-latest`: checkout, Node 22, Rust stable, `npm ci`,
  `cargo test`, `npm run tauri build`. Upload the installer (`src-tauri/target/release/bundle/**`)
  as an artifact so Yorgo can download it.
- Rename the product: `productName` "Pomodoro", window title "Pomodoro".

### Task 2: Live timer + tray

Files: `src-tauri/src/timer.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/commands.rs`, `lib.rs`,
`tauri.conf.json`, `capabilities/default.json`.

- Port `Sources/Pomodoro/TimerController.swift` (deadline based, so sleep or a slow tick never drifts;
  ticker every 500 ms on a background thread; elapse → next phase via engine, auto-start rules).
  Skip the widget mirror and command polling.
- Shared state in `Mutex` inside Tauri managed state. Emit the events in the contract.
- Tray (Windows notification area): Windows tray icons cannot show text, so the countdown goes in
  the tooltip and in a disabled first menu item ("Focus · 24:13"). Menu items match
  `MenuBarController.swift` / `MenuHeaderView.swift`: Start/Pause, Skip, Reset, Show floating bar,
  Settings…, Stats…, Quit. Update the tooltip and header each tick.
- No main window at startup and no taskbar button (the Mac app has no Dock icon). Remove the
  template's demo page and greet command.
- Single instance (`tauri-plugin-single-instance`).

### Task 3: Floating bar

Files: `src/routes/bar/+page.svelte`, `src/lib/ui/{TimerDial,CyclePips}.svelte`, `src/lib/theme.ts`,
`src/lib/timer.ts` (store fed by `timer-state`), window config in `tauri.conf.json` or created in Rust.

- Port `Sources/Pomodoro/FloatingBar.swift`, `PomodoroUI/TimerDial.swift`, `CyclePips.swift`,
  `Theme.swift`, `BarPresentation.swift` (shrink rules).
- Window: transparent, no frame, always on top, skip taskbar, not resizable, draggable by the pill.
  Glass look via CSS `backdrop-filter` plus Windows acrylic/mica if `window-vibrancy` crate works;
  fall back to a translucent fill.
- Save the position to settings (`floatingBarX/Y`) when a drag ends; restore it on launch.
- Respect `showFloatingBar` and `compactFloatingBar`.

### Task 4: Settings window + start at login

Files: `src/routes/settings/+page.svelte`, `src/lib/settings.ts`, Rust window opener, autostart.

- Port `Sources/Pomodoro/SettingsView.swift`: same sections, controls, ranges and labels.
- `launchAtLogin` uses `tauri-plugin-autostart` (port `LoginItem.swift`).
- Chime pickers list the sounds that Task 5 ships.

### Task 5: Notifications + chimes

Files: `src-tauri/src/alerts.rs`, sounds in `src-tauri/sounds/`, a small script that generates them.

- Port `Notifier.swift` (titles and bodies verbatim) with `tauri-plugin-notification`.
- Port `ChimePlayer.swift`. Windows has no "Hero"/"Blow". Ship a few short chimes generated by a
  committed script (sine/bell synthesis to WAV), named so the defaults read "Hero" and "Blow".
  Play with `rodio`. Volume 0 to 2 as in settings.

### Task 6: Stats

Files: `src-tauri/src/core/stats.rs`, `src/routes/stats/+page.svelte`.

- Port `StatsStore.swift` (same JSON shape, `stats.json` in the app data dir) and `StatsView.swift`.
- The timer records a finished focus session through it.

### Task 7: Music via SMTC

Files: `src-tauri/src/music.rs`, `Cargo.toml` (`windows` crate, target windows only).

- Port the rules of `MusicController.swift` onto SMTC: on focus end, pause the current session if it
  is playing and remember we did; on focus start, resume only the session we paused (if
  `resumeMusicOnFocusStart`). Respect `controlMusic`. Non-Windows build: no-op.

### Task 8: Character engine + samurai

Files: `src/lib/rig/{rig,timelines,movingHold,plan}.ts`, `src/lib/rig/Rig.svelte`,
`src/lib/characters/samurai/{art,performance}.ts`, `src/routes/stage/+page.svelte`, tests.

- Add vitest. Port `Rig.swift`, `RigTimelines.swift`, `MovingHold.swift`, `CharacterPlan.swift`,
  `CharacterStage.swift`, `RigSubject.swift` to TypeScript. Path data strings go straight into SVG `d`.
  Joint transforms: same order as the Swift (parent chain, rotate about pivot), as SVG `<g transform>`.
- Port `SamuraiArt.swift` + `SamuraiPerformance.swift` + `SamuraiView.swift`.
- Stage: a transparent, click-through, always-on-top window placed behind/above the bar (match how
  the Mac stage pops out of the pill and how `8d0cf97` measures room on the bar's display). On
  `phase-elapsed`, play the move for the new phase (focus start, break start, long break), then hide.
- Port the checks in `Tests/PomodoroUITests/RigTests.swift` that apply.

### Tasks 9 to 13: one character each

Same shape as the samurai, reusing the engine untouched:
9 ninja (`Ninja*.swift`), 10 rabbit (`Rabbit*.swift`), 11 general (`General*.swift`),
12 Power (`AnimeGirl*.swift`), 13 Boa Hancock (`Hancock*.swift`).
Register each in the character list so Settings can pick it.

### Task 14: README

Rewrite `README.md` for the Windows app, matching the Mac README's layout and tone; link the Mac repo.
List what differs on Windows (tray tooltip countdown, SMTC music, no widget).

## PR grouping

- PR 1 `port/timer`: Tasks 1 to 3.
- PR 2 `port/features`: Tasks 4 to 7.
- PR 3 `port/characters`: Task 8 (+ README note).
- PR 4 `port/cast`: Tasks 9 to 13.
- PR 5 `port/readme`: Task 14.
