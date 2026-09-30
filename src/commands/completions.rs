use std::error::Error;
use std::io;

use clap::{Arg, Command};
use clap_complete::{Shell, generate};

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
            Arg::new("app")
                .help("URL and name for the app to install")
                .index(1)
                .num_args(2)
                .value_names(["URL", "NAME"]),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .requires("app")
                .help("Overwrite an existing app")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("icon")
                .long("icon")
                .requires("app")
                .value_name("PATH")
                .help("Use a custom local icon instead of fetching"),
        )
        .arg(
            Arg::new("browser")
                .long("browser")
                .requires("app")
                .value_name("BROWSER")
                .help("Browser to use (e.g. chromium, brave-browser)"),
        )
        .arg(
            Arg::new("default-icon")
                .long("default-icon")
                .requires("app")
                .conflicts_with("icon")
                .help("Use the bundled icon without fetching")
                .action(clap::ArgAction::SetTrue),
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
                .conflicts_with_all(["app", "force", "icon", "browser", "default-icon"])
                .help("Run in interactive mode")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("list")
                .about("List or search installed apps")
                .arg(
                    Arg::new("query")
                        .help("Filter by name, URL, or stable slug")
                        .index(1),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Print complete app metadata as JSON")
                        .conflicts_with("names")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("names")
                        .long("names")
                        .help("Print one app name per line")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("open")
                .about("Open an installed app")
                .arg(Arg::new("name").required(true).index(1)),
        )
        .subcommand(
            Command::new("doctor")
                .about("Check saved URLs, browsers, and launcher/icon files")
                .arg(
                    Arg::new("name")
                        .help("Check one app (default: all)")
                        .index(1),
                ),
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
                        .conflicts_with_all(["new-name", "url", "browser", "icon", "default-icon"])
                        .help("Update all installed apps")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(Arg::new("new-name").long("name").value_name("NAME"))
                .arg(Arg::new("url").long("url").value_name("URL"))
                .arg(Arg::new("browser").long("browser").value_name("BROWSER"))
                .arg(Arg::new("icon").long("icon").value_name("PATH"))
                .arg(
                    Arg::new("default-icon")
                        .long("default-icon")
                        .conflicts_with("icon")
                        .help("Replace the icon with the bundled default")
                        .action(clap::ArgAction::SetTrue),
                ),
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
                .arg(
                    Arg::new("browser")
                        .long("browser")
                        .value_name("BROWSER")
                        .help("Use this browser for every restored app"),
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
    if shell == Shell::Fish {
        println!(
            "complete -c tack -n '__fish_seen_subcommand_from open remove update doctor; and test (__fish_number_of_cmd_args_wo_opts) -eq 2; and not __fish_seen_argument -l all' -f -a '(command tack list --names 2>/dev/null)'"
        );
    }
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
        assert_eq!(
            args.get_many::<String>("app").unwrap().next_back().unwrap(),
            "--quiet"
        );
        assert!(!args.get_flag("quiet"));
        for name in ["list", "open", "help", "config"] {
            let args = build_cli()
                .try_get_matches_from(["tack", "https://example.com", name])
                .unwrap();
            assert!(args.subcommand().is_none());
            assert_eq!(
                args.get_many::<String>("app").unwrap().next_back().unwrap(),
                name
            );
        }
    }
}
