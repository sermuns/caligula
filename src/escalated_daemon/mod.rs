//! This sounds satanic, but it's really just a background process running as root,
//! that spawns other processes running as root. The point is so that we can spawn
//! multiple writers running as root while only executing sudo once.
//!
//! The reason we can't just `sudo caligula` writer processes over and over again
//! is because most desktop sudo installations (rightfully) drop your sudo cookie
//! after a while and it would suck to have to repeatedly enter in your password
//! when you only really need to do it once.
//!
//! Given that this is running in root, we would like to restrict its interface as
//! much as possible. In the future, it may even be worthwhile to harden the IPC
//! even further.
//!
//! IT IS NOT TO BE USED DIRECTLY BY THE USER! ITS API HAS NO STABILITY GUARANTEES!

use anyhow::Context;
use interprocess::local_socket::{GenericFilePath, tokio::prelude::*};
use tokio::io::{AsyncBufRead, BufReader};
use tracing::{Instrument, error, info, info_span};
use tracing_unwrap::ResultExt;

use crate::{childproc_common::child_init, ipc_common::read_msg_async};

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    todo!()
}