# tack

> A CLI tool to install any website as a desktop app on Linux using your system's Chromium.

## How it works

`tack` takes a website URL and a name, and automatically sets up a desktop application for it. It features a robust icon fetching pipeline: it first checks your local icon cache, then tries to fetch high-quality SVG logos from `svgl.app`, scrapes the website's HTML for `<link>` tags (like `apple-touch-icon`), and finally falls back to `/favicon.ico` or the Google Favicons API. It supports `.png`, `.svg`, and even legacy `.ico` formats (which it natively converts to `.png`). It then generates a `.desktop` file that launches the website using your browser in a standalone app window. All installed applications are tracked in a metadata manifest.

It can auto-detect installed browsers by scanning your `PATH` and handles specific command line flags automatically (like `--app=` for Chromium-based or `--ssb` for Firefox-based browsers).

## Usage

```bash
tack <url> <name> [--force] [--icon PATH] [--browser BROWSER] [--dry-run] [--quiet] [--verbose]
tack -i                           # interactive mode
tack list
tack open <name>
tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH] [--dry-run]
tack update --all
tack remove <name>
tack export [file]
tack import <file>
tack config show
tack config set <key> <value>
```

### Install an App

To install YouTube as a desktop app:

```bash
tack https://youtube.com YouTube
```

This will create a "YouTube" application in your app launcher. `tack` will normalize URLs (adding `https://` if missing), validate them, and sanitize app names. If an app with the same name is already installed, it will exit unless you provide the `--force` flag to overwrite it.

You can also bypass the automatic icon fetching by providing a custom icon path:

```bash
tack https://youtube.com YouTube --icon /path/to/my-icon.png
```

To specify a browser explicitly instead of relying on auto-detection:

```bash
tack https://youtube.com YouTube --browser firefox
```

### Interactive Mode

For a guided setup, use `-i` to be prompted step-by-step for the URL, name, browser, and icon:

```bash
tack -i
```

This will show detected browsers as a numbered list and let you choose an icon source (fetch from URL, custom path, or default).

### Dry Run

Preview what `tack` would do without writing anything to disk:

```bash
tack https://youtube.com YouTube --dry-run
```

### Output Control

Control how much output `tack` produces:

```bash
tack https://youtube.com YouTube --quiet     # suppress stdout, errors still go to stderr
tack https://youtube.com YouTube --verbose   # detailed logs (paths, HTTP status, format detection)
```

### List Installed Apps

To list all applications currently installed and managed by `tack`:

```bash
tack list
```

### Open an App

To launch a previously installed application from the terminal:

```bash
tack open YouTube
```

### Update an App

To modify an existing application (e.g., change its name, URL, browser, or icon):

```bash
tack update YouTube --name "YouTube Music" --url https://music.youtube.com
```

If no flags are provided, `tack update` runs in "repair mode", which re-fetches the favicon and regenerates the `.desktop` file.

To update all applications at once (e.g. re-fetching missing icons and regenerating desktop files for every installed app):

```bash
tack update --all
```

### Export and Import

You can export your installed apps manifest as a portable JSON file, and restore them on another machine:

```bash
# Dump JSON to stdout
tack export

# Save to a file
tack export backup.json

# Restore from backup file
tack import backup.json
```

### Remove an App

To remove an installed application:

```bash
tack remove YouTube
```

This removes the `.desktop` file, the saved icon (if managed by tack), and the app's entry from the manifest.

### Manage Configuration

You can use `tack config` to manage default behaviors like the default browser and default categories for the generated `.desktop` files. The config is saved at `~/.config/tack/config.toml`.

To show the current configuration:
```bash
tack config show
```

To update a configuration value:
```bash
tack config set browser brave-browser
tack config set categories "Network;Entertainment;"
```

## Features

- **Auto icon fetching** — svgl.app → HTML `<link>` tags → `/favicon.ico` → Google Favicons API
- **ICO to PNG conversion** — native, no external tools
- **Browser auto-detection** — scans `PATH` for Chromium/Firefox-based browsers
- **Colored output** — green/yellow/red ANSI colors, respects `NO_COLOR`
- **URL validation** — catches malformed URLs before any work is done
- **Offline detection** — fast TCP check before attempting network requests
- **Interactive mode** (`-i`) — guided step-by-step setup
- **Dry run** (`--dry-run`) — preview changes without touching the filesystem
- **Quiet/Verbose** (`--quiet`, `--verbose`) — control output verbosity
- **Persistent config** — `~/.config/tack/config.toml` for defaults

## Requirements

- Linux
- A supported browser (Chromium-based or Firefox-based) installed on your system
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

- `~/.local/share/applications/<slug>.desktop`
- `~/.local/share/icons/<slug>.{png,svg}`
- `~/.local/share/tack/apps.json` (apps tracking manifest)

## License

[MIT](LICENSE)
