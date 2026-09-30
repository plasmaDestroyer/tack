# tack

> A CLI tool to install any website as a desktop app on Linux using your system's Chromium.

## How it works

`tack` takes a website URL and a name, and automatically sets up a desktop application for it. It features a robust icon fetching pipeline: it first checks your local icon cache, then tries to fetch high-quality SVG logos from `svgl.app`, scrapes the website's HTML for `<link>` tags (like `apple-touch-icon`), and finally falls back to `/favicon.ico` or the Google Favicons API. It supports `.png`, `.svg`, and even legacy `.ico` formats (which it natively converts to `.png`). It then generates a `.desktop` file that launches the website using your browser in a standalone app window. All installed applications are tracked in a metadata manifest.

It auto-detects Chromium-based browsers on your `PATH` and opens sites with the `--app=` flag. Firefox and Zen do not support standalone app windows on Linux.

## Usage

```bash
tack <url> <name> [--force] [--icon PATH] [--browser BROWSER] [--dry-run] [--quiet] [--verbose]
tack -i                           # interactive mode
tack list [query] [--json | --names]
tack open <name>
tack update <name> [--name NAME] [--url URL] [--browser BROWSER] [--icon PATH] [--dry-run]
tack update --all
tack remove <name> [--dry-run]
tack export [file] [--no-icons]
tack import <file> [--browser BROWSER]
tack completions <bash|zsh|fish>
tack manpage
tack config show
tack config set <key> <value>
tack --version
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
tack https://youtube.com YouTube --browser brave-browser
```

### Interactive Mode

For a guided setup, use `-i` to be prompted step-by-step for the URL, name, browser, and icon:

```bash
tack -i
```

This shows detected browsers as a numbered list, with your configured browser first, and lets you choose an icon source (fetch from URL, custom path, or default). Invalid entries prompt again; closing input cancels setup. Quiet mode is unavailable during guided setup.

### Dry Run

Add `--dry-run` to install, update, remove, import, export, config set, open, or interactive mode to preview without writing files or launching a browser:

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

Search by name, URL, or stable slug with `tack list music`. Default output keeps long rows compact; `tack list --verbose` shows full paths and metadata. Use `tack list --json` for scripts or `tack list --names` for one name per line.

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

You can export your installed apps and icons as JSON and recreate the apps on another machine:

```bash
# Dump JSON to stdout
tack export

# Save to a file
tack export backup.json

# Restore from backup file
tack import backup.json
```

Icons are embedded as JSON byte arrays, including custom icons. Import restores them into the new machine's data directory and preserves custom icons during later repairs. If the saved browser paths differ, use `tack import backup.json --browser chromium` to select the destination's browser for all apps.

Older metadata-only backups still work; import re-fetches icons or uses cached ones when no image is embedded. Use `tack export backup.json --no-icons` for a smaller metadata-only backup. Export fails if a custom icon cannot be read, so it cannot silently lose your image.

### Remove an App

To remove an installed application:

```bash
tack remove YouTube
```

This removes the `.desktop` file, the saved icon (if managed by tack), and the app's entry from the manifest.

Install, update, and remove also clean up unused PNG/SVG variants left by older Tack versions for that app. Run `tack update --all` to clean existing apps; custom icons are preserved.

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

### Shell Completions

Generate tab-completion scripts for your shell:

```bash
# Bash — append to ~/.bashrc
tack completions bash >> ~/.bashrc

# Zsh — add to fpath
tack completions zsh > ~/.zfunc/_tack

# Fish
tack completions fish > ~/.config/fish/completions/tack.fish
```

### Man Page

View or install the man page:

```bash
tack manpage | man -l -                                  # view directly
tack manpage | sudo tee /usr/local/share/man/man1/tack.1  # install
```

## Features

- **Auto icon fetching** — svgl.app → HTML `<link>` tags → `/favicon.ico` → Google Favicons API
- **ICO to PNG conversion** — native, no external tools
- **Browser auto-detection** — scans `PATH` for Chromium-based browsers
- **Colored output** — green/yellow/red ANSI colors, respects `NO_COLOR`
- **URL validation** — catches malformed URLs before any work is done
- **Bounded icon fetches** — short request timeouts; falls back to the default icon if fetching fails
- **Interactive mode** (`-i`) — guided step-by-step setup
- **Dry run** (`--dry-run`) — preview changes without touching the filesystem
- **Quiet/Verbose** (`--quiet`, `--verbose`) — control output verbosity
- **Persistent config** — `~/.config/tack/config.toml` for defaults
- **Export/Import** — portable JSON backup and restore
- **Shell completions** — bash, zsh, fish via `tack completions`
- **Man page** — auto-generated via `tack manpage`

## Requirements

- Linux
- A Chromium-based browser installed on your system
- Rust and Cargo (for building from source)

## Installation

### AUR (Arch Linux)

```bash
yay -S tack-cli
```

### Build from source

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
