use std::fs::File;

use crate::{
    herder::Herder,
    logging::LogFile,
    tty::TermiosRestore,
    ui::{
        cli::BurnArgs,
        simple_ui::do_setup_wizard,
        start::{begin_writing, try_start_burn},
    },
};
use inquire::InquireError;
use tracing::{debug, info};

pub async fn main(args: &BurnArgs, log_file: LogFile) {
    let _termios_restore = match File::open("/dev/tty") {
        Ok(tty) => TermiosRestore::new(tty).ok(),
        Err(error) => {
            info!(
                ?error,
                "failed to open /dev/tty, will not attempt to restore after program"
            );
            None
        }
    };

    debug!("Starting primary process");
    match inner_main(args, log_file).await {
        Ok(_) => (),
        Err(e) => handle_toplevel_error(e),
    }
}

fn handle_toplevel_error(err: anyhow::Error) {
    if let Some(e) = err.downcast_ref::<InquireError>() {
        match e {
            InquireError::OperationCanceled
            | InquireError::OperationInterrupted
            | InquireError::NotTTY => eprintln!("{e}"),
            _ => panic!("{err}"),
        }
    } else {
        panic!("{err}");
    }
}

async fn inner_main(args: &BurnArgs, log_file: LogFile) -> anyhow::Result<()> {
    let Some(begin_params) = do_setup_wizard(&args)? else {
        return Ok(());
    };

    let mut herder = Herder::new(log_file);
    let handle = try_start_burn(
        &mut herder,
        &begin_params.make_child_config(),
        args.root,
        args.interactive.is_interactive(),
    )
    .await?;
    begin_writing(args.interactive, begin_params, handle, log_paths).await?;

    debug!("Done!");
    Ok(())
}
