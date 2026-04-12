use clap::Parser;
use tracing::info;

use crate::{
    cli::CaligulaArgs,
    logging::{setup_child_logging, setup_parent_logging},
};

mod byteseries;
mod cli;
mod compression;
mod device;
mod escalation;
mod hash;
mod hashfile;
mod herder;
mod ipc_common;
mod logging;
mod native;
mod tty;
mod ui;
mod util;

fn main() {
    let args = CaligulaArgs::parse();

    match args.command {
        cli::Command::Burn(_burn_args) => {
            setup_parent_logging();
            todo!()
        }
        cli::Command::RemoteHerder(_herder_args) => {
            setup_child_logging();
            info!("Initializing herder");
            todo!()
        }
    }
}
