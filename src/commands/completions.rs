use std::error::Error;
use std::io;

use clap::{Arg, Command};
use clap_complete::{Shell, generate};

use crate::output;

/// Build a clap `Command` that mirrors tack's CLI surface.
/// This is used for shell-completion and man-page generation —
/// actual argument parsing stays hand-rolled in main.rs.
pub fn build_cli() -> Command {
    Command::new("tack")
        .about("Install any website as a desktop app on Linux")
        .long_about(
            "tack takes a website URL and a name, and automatically sets up a desktop \
             application for it. It fetches icons, generates .desktop files, and tracks \
             installed apps in a manifest.",
        )
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(false)
        .arg(Arg::new("url").help("URL to install").index(1))
        .arg(Arg::new("name").help("Name for the app").index(2))
        .arg(
            Arg::new("force")
                .long("force")
                .help("Overwrite an existing app")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("icon")
                .long("icon")
                .value_name("PATH")
                .help("Use a custom local icon instead of fetching"),
        )
        .arg(
            Arg::new("browser")
                .long("browser")
                .value_name("BROWSER")
                .help("Browser to use (e.g. chromium, firefox)"),
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
                .arg(Arg::new("name").index(1))
                .arg(
                    Arg::new("all")
                        .long("all")
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
                .about("Export manifest as JSON")
                .long_about("Dump the apps manifest as portable JSON to stdout or a file.")
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
                    "Restore from an exported JSON file, re-fetch icons and \
                     recreate .desktop files for each entry.",
                )
                .arg(Arg::new("file").required(true).index(1)),
        )
        .subcommand(
            Command::new("config")
                .about("Manage tack configuration")
                .subcommand(Command::new("show").about("Show current config"))
                .subcommand(
                    Command::new("set")
                        .about("Set a config value")
                        .arg(Arg::new("key").required(true).index(1))
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

pub fn generate_completions(shell_name: &str) -> Result<(), Box<dyn Error>> {
    let shell: Shell = shell_name
        .parse()
        .map_err(|_| format!("Unsupported shell: {}. Use bash, zsh, or fish.", shell_name))?;

    let mut cmd = build_cli();
    generate(shell, &mut cmd, "tack", &mut io::stdout());

    output::verbose(&format!("Generated {} completions.", shell_name));
    Ok(())
}

pub fn generate_manpage() -> Result<(), Box<dyn Error>> {
    let cmd = build_cli();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut io::stdout())?;
    Ok(())
}
