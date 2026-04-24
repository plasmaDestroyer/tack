# tack

> A CLI tool to install any website as a desktop app on Linux using your system's Chromium.

## How it works

`tack` takes a website URL and a name, and automatically sets up a desktop application for it. It features a robust icon fetching pipeline: it first checks your local icon cache, then tries to fetch high-quality SVG logos from `svgl.app`, scrapes the website's HTML for `<link>` tags (like `apple-touch-icon`), and finally falls back to `/favicon.ico` or the Google Favicons API. It supports `.png`, `.svg`, and even legacy `.ico` formats (which it natively converts to `.png`). It then generates a `.desktop` file that launches the website using your browser in a standalone app window. All installed applications are tracked in a metadata manifest.

It can auto-detect installed browsers by scanning your `PATH` and handles specific command line flags automatically (like `--app=` for Chromium-based or `--ssb` for Firefox-based browsers).

## Usage

```bash
tack <url> <name> [--force] [--icon PATH] [--browser BROWSER]
tack list
tack open <name>
tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH]
tack remove <name>
tack config show
tack config set <key> <value>
```

### Install an App

To install YouTube as a desktop app:

```bash
tack https://youtube.com YouTube
```

This will create a "YouTube" application in your app launcher. `tack` will normalize URLs (adding `https://` if missing) and sanitize app names. If an app with the same name is already installed, it will exit unless you provide the `--force` flag to overwrite it.

You can also bypass the automatic icon fetching by providing a custom icon path:

```bash
tack https://youtube.com YouTube --icon /path/to/my-icon.png
```

To specify a browser explicitly instead of relying on auto-detection:

```bash
tack https://youtube.com YouTube --browser firefox
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
