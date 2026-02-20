# Slipbridge

An agent-first CLI interface for thermal receipt printers.

Slipbridge bridges the gap between AI agents and physical thermal printers, providing a composable command-line tool for formatting and printing receipts via ESC/POS-compatible hardware.

## Status

Early development. Core architecture is being built out.

## Planned Features

- ESC/POS command generation and printer communication
- Structured receipt formatting from JSON/stdin
- Agent-friendly interface (machine-readable output, piping support)
- USB and network printer discovery
- Template system for common receipt layouts

## Development

### Prerequisites

- Rust toolchain (`cargo`, `rustc`)
- [`just`](https://github.com/casey/just) (optional, recommended for task shortcuts)

### Common Commands

Using `just`:

```bash
just check
just fmt
just lint
just test
just install-local
just discover-json
just smoke
```

Direct `cargo` equivalents:

```bash
cargo check --all-targets
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo install --path . --force
```

### Printer Testing

Smoke test a specific printer:

```bash
just smoke printer="tcp://host-or-mdns.local:9100"
```

Print an image and force cut:

```bash
just print-image /absolute/path/to/image.png printer="tcp://host-or-mdns.local:9100"
```

### Updating Local Install

Rebuild and reinstall the current branch:

```bash
just install-local
```

Switch to a branch, pull latest, and reinstall:

```bash
just update-install branch="codex/m1-foundation"
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
