use std::sync::Arc;

use super::Herder;
use super::{LocalHerderSocket, WriterHandle, handle::ChildHandle};
use crate::ipc_common::write_msg_async;
use crate::logging::LogPaths;
use crate::{
    ipc_common::read_msg_async,  writer_process::ipc::ErrorType,
};
use anyhow::Context;
use interprocess::local_socket::tokio::prelude::*;
use tracing::{debug, trace};

use crate::escalation::run_escalate;
use crate::writer_process::ipc::{StatusMessage, WriterProcessConfig};

/// A [Herder] that runs threads on this local process.
pub struct LocalHerder {}

impl LocalHerder {
    pub fn new() -> Self {
        Self {}
    }
}

impl Herder for LocalHerder {
    fn spawn_action_group(
        &self,
        params: &super::data::SpawnActionGroupRequest,
    ) -> impl Future<
        Output = Result<super::data::SpawnActionGroupResponse, super::data::SpawnActionGroupError>,
    > {
        todo!()
    }

    fn stop_action_group(
        &self,
        params: &super::data::StopActionGroupRequest,
    ) -> impl Future<
        Output = Result<super::data::StopActionGroupResponse, super::data::StopActionGroupError>,
    > {
        todo!()
    }
}
