use clap::Parser;

use crate::cli::{BurnArgs, CaligulaArgs};

mod byteseries;
mod childproc_common;
mod cli;
mod compression;
mod device;
mod escalation;
mod hash;
mod hashfile;
mod ipc_common;
mod logging;
mod native;
mod tty;
mod ui;
mod util;

fn main() {
    let args = CaligulaArgs::parse();

    match args.command {
        cli::Command::Burn(_burn_args) => todo!(),
        cli::Command::RemoteHerder(_herder_args) => todo!(),
    }
}
