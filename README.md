# tack

> A CLI tool to install any website as a desktop app on Linux using your system's Chromium.

## How it works

`tack` takes a website URL and a name, and automatically sets up a desktop application for it. It fetches the website's favicon, saves it to your system icons directory, and generates a `.desktop` file that launches the website using Chromium in a standalone app window.

## Usage

```bash
tack <url> <name>
```

### Example

To install YouTube as a desktop app:

```bash
tack https://youtube.com YouTube
```

This will create a "YouTube" application in your app launcher.

## Requirements

- Linux
- Chromium installed on your system
- Rust and Cargo (for building from source)

## Installation

Build from source:

```bash
git clone https://github.com/plasmaDestroyer/tack.git
cd tack
cargo build --release
# The executable will be available at target/release/tack
```

## What it creates

- `~/.local/share/applications/<name>.desktop`
- `~/.local/share/icons/<name>.png`

## License

[MIT](LICENSE)
