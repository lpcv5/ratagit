# ratagit

A keyboard-driven Git TUI built with Rust and ratatui, aiming for lazygit-style
workflows.

Run from a Git working directory with stable Rust and Git installed:

```sh
cargo run --locked --bin ratagit
```

Use `?` to open the command palette.

- [Themes and configuration](docs/THEMES.md): presets, JSON color overrides,
  configuration locations, and contrast constraints.
- [Product behavior](docs/PRODUCT.md): panels, navigation, and Git workflows.
- [Design](docs/DESIGN.md) and [architecture](ARCHITECTURE.md).
- [Development workflow](docs/DEVELOPMENT_LOOP.md) and [testing](docs/TESTING.md).
