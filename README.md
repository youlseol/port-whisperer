# port-whisperer

Rust-based CLI for inspecting listening ports, dev processes, and orphaned servers on macOS and Windows.

The canonical implementation now lives in [`rust/`](/Users/A59833/Documents/Workspace/port-whisperer/rust). The root-level JavaScript files under [`src/`](/Users/A59833/Documents/Workspace/port-whisperer/src) are kept as a legacy snapshot and are no longer the product entrypoint.

## Layout

- [`rust/`](/Users/A59833/Documents/Workspace/port-whisperer/rust): active implementation, binaries, tests
- [`src/`](/Users/A59833/Documents/Workspace/port-whisperer/src): legacy Node implementation
- [`tasks/`](/Users/A59833/Documents/Workspace/port-whisperer/tasks): local planning and review notes

## Install

Build and install both `ports` and `whoisonport` with Cargo:

```bash
cargo install --path rust
```

Run locally without installing:

```bash
cargo run --manifest-path rust/Cargo.toml --bin ports -- --help
cargo run --manifest-path rust/Cargo.toml --bin ports --
cargo run --manifest-path rust/Cargo.toml --bin ports -- --plain
```

Windows PowerShell uses the same Rust commands:

```powershell
cargo install --path rust
ports
ports ps
ports 3000
ports clean
ports watch
```

## Usage

```bash
ports
ports --all
ports --interval-ms 1000
ports ps
ports 3000
ports clean
ports watch
ports --plain ps
whoisonport 3000
```

## TUI

- `ports` now opens a full-screen `ratatui` dashboard by default when stdin/stdout is a TTY.
- Use `--plain` to force the legacy text renderer or when piping output.
- Main keys:
  - `j`/`k`, arrow keys: move selection
  - `Tab` / `Shift+Tab`: switch views
  - `/`: filter
  - `s`: cycle sort
  - `a`: toggle dev-only vs all scope for the current view
  - `x`: kill selected process
  - `c`: clean orphaned/zombie targets
  - `r`: refresh
  - `d` or `Enter`: expand/collapse detail emphasis
  - `q`: quit

## Verification

```bash
cargo test --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml -- -D warnings
```

## Platform Support

| Platform | Status |
|----------|--------|
| macOS    | Supported |
| Windows  | Supported |
| Linux    | Planned |

## Notes

- Framework and project detection are based on process `cwd` plus project markers like `package.json`, `Cargo.toml`, and `pyproject.toml`.
- Runtime install directories are intentionally not treated as project roots.
- `watch` is polling-based.
- Non-interactive environments automatically fall back to plain text output.
- Some kill operations still require elevated permissions, especially on Windows.

## License

[MIT](LICENSE)
