use bimap::BiMap;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::Herder;
use super::{LocalHerderSocket, WriterHandle, handle::ChildHandle};
use crate::herder::data::{ActionGroupId, HerderEvent};
use crate::ipc_common::write_msg_async;
use crate::logging::LogPaths;
use crate::{
    ipc_common::read_msg_async, writer_process::ipc::ErrorType,
};
use anyhow::Context;
use interprocess::local_socket::tokio::prelude::*;
use tracing::{debug, trace};

use crate::escalation::run_escalate;
use crate::writer_process::ipc::{StatusMessage, WriterProcessConfig};

/// A [Herder] that tries to spawn the first one, then delegates to the second one if
/// permissions are lacking.
pub struct FallbackHerder<H1: Herder, H2: Herder> {
    first: H1,
    second: H2,
}

impl<H1: Herder, H2: Herder> FallbackHerder<H1, H2> {
    pub fn new(first: H1, second: H2) -> Self {
        Self { first, second }
    }
}

impl<H1: Herder, H2: Herder> Herder for FallbackHerder<H1, H2> {
    fn spawn_action_group(
        &self,
        params: &super::data::SpawnActionGroupRequest,
    ) -> impl Future<
        Output = Result<super::data::SpawnActionGroupResponse, super::data::SpawnActionGroupError>,
    > {
        async {
            match self.first.spawn_action_group(params).await {
                Ok(mut r) => return Ok(r),
                Err(super::data::SpawnActionGroupError::PermissionDenied) => {
                    self.second.spawn_action_group(params).await
                }
            }
        }
    }

    fn stop_action_group(
        &self,
        params: &super::data::StopActionGroupRequest,
    ) -> impl Future<
        Output = Result<super::data::StopActionGroupResponse, super::data::StopActionGroupError>,
    > {
        async {
            match self.first.stop_action_group(params).await {
                Ok(mut r) => return Ok(r),
                Err(super::data::StopActionGroupError::UnknownActionGroup(_)) => {
                    self.second.stop_action_group(params).await
                }
            }
        }
    }
}
