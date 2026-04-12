use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use tracing::info;

use crate::ui::BurnArgs;

mod byteseries;
mod compression;
mod device;
mod escalated_daemon;
mod escalation;
mod evdist;
mod hash;
mod hashfile;
mod herder;
mod ipc_common;
mod logging;
mod native;
mod tty;
mod ui;
mod writer_process;

/// A lightweight, user-friendly disk imaging tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, flatten_help = true)]
#[command(propagate_version = true)]
pub struct CaligulaArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Burn(BurnArgs),

    #[command(name = "_herder", hide = true)]
    RemoteHerder,
}

#[tokio::main]
async fn main() {
    let args: CaligulaArgs = match std::env::var("_CALIGULA_CONFIGURE_CLAP_FOR_README") {
        Ok(var) if var == "1" => parse_args_for_readme_generation(),
        _ => CaligulaArgs::parse(),
    };

    match args.command {
        Command::Burn(burn_args) => {
            let log_file = logging::setup_parent_logging();
            ui::main(&burn_args, log_file);
        }
        Command::RemoteHerder => {
            logging::setup_child_logging();
            info!("Initializing herder");
            escalated_daemon::main();
        }
    }
}

/// Parse [CaligulaArgs] from the provided args, but format the help in an easy way for generating
/// the section in the README.md.
fn parse_args_for_readme_generation() -> CaligulaArgs {
    let command = CaligulaArgs::command_for_update()
        .color(clap::ColorChoice::Never)
        .term_width(0);

    // The rest of this function is lifted out of clap::Parser::parse().
    let mut matches = command.get_matches();
    let res = CaligulaArgs::from_arg_matches_mut(&mut matches).map_err(|err| {
        let mut cmd = CaligulaArgs::command();
        err.format(&mut cmd)
    });
    match res {
        Ok(s) => s,
        Err(e) => {
            // Since this is more of a development-time error, we aren't doing as fancy of a quit
            // as `get_matches`
            e.exit()
        }
    }
}
