use std::error::Error;
use std::io;

use clap::{Arg, Command};
use clap_complete::{Shell, generate};

use crate::output;

/// Shared CLI definition for parsing, help, completions, and man pages.
pub fn build_cli() -> Command {
    Command::new("tack")
        .about("Install any website as a desktop app on Linux")
        .long_about(
            "tack takes a website URL and a name, and automatically sets up a desktop \
             application for it. It fetches icons, generates .desktop files, and tracks \
             installed apps in a manifest.",
        )
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("url")
                .help("URL to install")
                .index(1)
                .requires("name"),
        )
        .arg(
            Arg::new("name")
                .help("Name for the app")
                .index(2)
                .requires("url"),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .requires("url")
                .help("Overwrite an existing app")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("icon")
                .long("icon")
                .requires("url")
                .value_name("PATH")
                .help("Use a custom local icon instead of fetching"),
        )
        .arg(
            Arg::new("browser")
                .long("browser")
                .requires("url")
                .value_name("BROWSER")
                .help("Browser to use (e.g. chromium, brave-browser)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .global(true)
                .help("Preview changes without writing to disk")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .conflicts_with("verbose")
                .global(true)
                .help("Suppress non-error output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .global(true)
                .help("Show detailed logs")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("interactive")
                .long("interactive")
                .short('i')
                .conflicts_with_all(["url", "name", "force", "icon", "browser"])
                .help("Run in interactive mode")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(Command::new("list").about("List installed apps"))
        .subcommand(
            Command::new("open")
                .about("Open an installed app")
                .arg(Arg::new("name").required(true).index(1)),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove an installed app")
                .arg(Arg::new("name").required(true).index(1)),
        )
        .subcommand(
            Command::new("update")
                .about("Update an installed app or all apps")
                .long_about(
                    "Update a specific app's name, URL, browser, or icon. \
                     With --all, re-fetch favicons and rewrite .desktop files \
                     for every installed app.",
                )
                .arg(
                    Arg::new("name")
                        .index(1)
                        .required_unless_present("all")
                        .conflicts_with("all"),
                )
                .arg(
                    Arg::new("all")
                        .long("all")
                        .conflicts_with_all(["new-name", "url", "browser", "icon"])
                        .help("Update all installed apps")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(Arg::new("new-name").long("name").value_name("NAME"))
                .arg(Arg::new("url").long("url").value_name("URL"))
                .arg(Arg::new("browser").long("browser").value_name("BROWSER"))
                .arg(Arg::new("icon").long("icon").value_name("PATH")),
        )
        .subcommand(
            Command::new("export")
                .about("Export apps and icons as JSON")
                .long_about(
                    "Back up app metadata and available icons as JSON to stdout or a file. \
                     Use --no-icons for a metadata-only export.",
                )
                .arg(
                    Arg::new("no-icons")
                        .long("no-icons")
                        .help("Export metadata without icon files")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("file")
                        .index(1)
                        .help("Output file (default: stdout)"),
                ),
        )
        .subcommand(
            Command::new("import")
                .about("Import apps from JSON file")
                .long_about(
                    "Restore apps and embedded icons from an exported JSON file. \
                     Older metadata-only backups remain supported; missing icons are re-fetched.",
                )
                .arg(Arg::new("file").required(true).index(1)),
        )
        .subcommand(
            Command::new("config")
                .about("Manage tack configuration")
                .subcommand_required(true)
                .subcommand(Command::new("show").about("Show current config"))
                .subcommand(
                    Command::new("set")
                        .about("Set a config value")
                        .arg(
                            Arg::new("key")
                                .required(true)
                                .index(1)
                                .value_parser(["browser", "categories"]),
                        )
                        .arg(Arg::new("value").required(true).index(2)),
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg(
                    Arg::new("shell")
                        .required(true)
                        .index(1)
                        .value_parser(clap::value_parser!(Shell)),
                ),
        )
        .subcommand(Command::new("manpage").about("Generate man page and print to stdout"))
}

pub fn generate_completions(shell: Shell) {
    let mut cmd = build_cli();
    generate(shell, &mut cmd, "tack", &mut io::stdout());

    output::verbose(&format!("Generated {} completions.", shell));
}

pub fn generate_manpage() -> Result<(), Box<dyn Error>> {
    let cmd = build_cli();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut io::stdout())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_enforces_command_requirements_and_keeps_literal_values() {
        build_cli().debug_assert();
        for args in [
            vec!["tack", "https://example.com"],
            vec!["tack", "--icon", "icon.png"],
            vec!["tack", "-i", "https://example.com", "Demo"],
            vec!["tack", "-q", "--verbose", "list"],
            vec!["tack", "--quiet", "-v", "list"],
            vec!["tack", "update"],
            vec!["tack", "update", "Demo", "--all"],
            vec!["tack", "update", "--all", "--icon", "icon.png"],
            vec!["tack", "config"],
            vec!["tack", "config", "set", "typo", "value"],
        ] {
            assert!(build_cli().try_get_matches_from(&args).is_err(), "{args:?}");
        }
        let args = build_cli()
            .try_get_matches_from(["tack", "https://example.com", "--", "--quiet"])
            .unwrap();
        assert_eq!(args.get_one::<String>("name").unwrap(), "--quiet");
        assert!(!args.get_flag("quiet"));
    }
}
