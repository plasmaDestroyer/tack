use std::error::Error;
use std::io;

use clap::{Arg, Command};
use clap_complete::{Shell, generate};

use crate::output;

/// Build a clap `Command` that mirrors tack's CLI surface.
/// This is used exclusively for shell-completion generation —
/// actual argument parsing stays hand-rolled in main.rs.
pub fn build_cli() -> Command {
    Command::new("tack")
        .about("Install any website as a desktop app on Linux")
        .subcommand_required(false)
        .arg(Arg::new("url").help("URL to install").index(1))
        .arg(Arg::new("name").help("Name for the app").index(2))
        .arg(
            Arg::new("force")
                .long("force")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(Arg::new("icon").long("icon").value_name("PATH"))
        .arg(Arg::new("browser").long("browser").value_name("BROWSER"))
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .global(true)
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .global(true)
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .global(true)
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("interactive")
                .long("interactive")
                .short('i')
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
                .arg(Arg::new("name").index(1))
                .arg(Arg::new("all").long("all").action(clap::ArgAction::SetTrue))
                .arg(Arg::new("new-name").long("name").value_name("NAME"))
                .arg(Arg::new("url").long("url").value_name("URL"))
                .arg(Arg::new("browser").long("browser").value_name("BROWSER"))
                .arg(Arg::new("icon").long("icon").value_name("PATH")),
        )
        .subcommand(
            Command::new("export")
                .about("Export manifest as JSON")
                .arg(Arg::new("file").index(1)),
        )
        .subcommand(
            Command::new("import")
                .about("Import apps from JSON file")
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
