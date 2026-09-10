# worktimeTUI

A local Rust + Ratatui project timer using the CyberNord ANSI palette.

## Run

```sh
cargo run --release --locked
```

Or install with `cargo install --path . --locked`, then run `worktimeTUI`.
Requires a current stable Rust toolchain and a true-color terminal at least
65 columns × 27 rows.

## Controls

| Key / action | Behavior |
| --- | --- |
| N | Create a named project; Enter saves, Esc cancels |
| ↑ / ↓ or K / J | Open a saved project |
| Click a project | Open that project |
| Space / Enter / click START | Start or pause |
| S | End current session, retaining recorded totals |
| P | Switch stopwatch / Pomodoro; stops current session |
| B | Include or exclude accumulated breaks in displayed totals |
| Q / Ctrl+C | Save and quit |

Switching projects stops the old session and opens the new project paused. There
is only one active timer. Names must be unique (case-insensitive), with a maximum
of 64 characters. Arrow keys also scroll long project lists.

Pomodoro automatically cycles through 25 minutes focus and 5 minutes break, with
a 15-minute break after every fourth focus round. Pause freezes the current phase.
Stopping, switching projects, or changing modes resets the current cycle, never
previously recorded time. Focus and break totals are stored separately. The B
setting applies globally and retroactively to displayed totals; breaks are
excluded by default. Manual pauses are never counted.

## Storage and recovery

Data lives in `$XDG_DATA_HOME/worktimeTUI/projects.json`, falling back to
`~/.local/share/worktimeTUI/projects.json`. It is independent of this checkout.
Use `worktimeTUI --data-dir /path/to/data` for a separate dataset.

Writes occur every second and on user actions, using a synced temporary file and
atomic rename. An OS file lock prevents concurrent writers. Invalid or unsupported
data causes startup to fail without replacing the existing file. Back up the JSON
file to preserve projects, totals, and preferences.

Startup always pauses timers. App downtime is never counted; a crash may lose the
last second of time. The monotonic timer avoids changes to the wall clock. On Linux,
computer suspend time is excluded. A blocked but awake process catches up elapsed
time and allocates it across focus and break boundaries.

This first version stores cumulative totals, not dated session history. Pomodoro
lengths are fixed; phase changes are visual, with no notifications yet.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

The UI uses [Ratatui](https://docs.rs/ratatui/0.30.2/ratatui/).
The palette follows the supplied CyberNord16colorANSI reference: background
`#2e3440`, borders `#0077b6`, accent `#00e5ff`, text `#e5e9f0`, focus
`#a3be8c`, and break `#b48ead`.
