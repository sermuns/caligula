use std::sync::Arc;

use super::{LocalHerderSocket, WriterHandle, handle::ChildHandle};
use super::Herder;
use crate::ipc_common::write_msg_async;
use crate::logging::LogPaths;
use crate::{
    ipc_common::read_msg_async, writer_process::ipc::ErrorType,
};
use anyhow::Context;
use interprocess::local_socket::tokio::prelude::*;
use tracing::{debug, trace};

use crate::escalation::run_escalate;

/// A [Herder] that runs threads on this local process.
pub struct RemoteHerder {
}

impl RemoteHerder {
    pub fn new() -> Self {
        Self {
        }
    }

}