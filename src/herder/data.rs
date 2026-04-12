use serde::{Deserialize, Serialize};

use crate::writer_process::ipc::WriterProcessConfig;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug, Serialize, Deserialize)]
pub struct ActionGroupId(pub u64);

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SpawnActionGroupRequest {
    pub ag: ActionGroupId,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SpawnActionGroupResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum SpawnActionGroupError {
    PermissionDenied,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StopActionGroupRequest {
    pub ag: ActionGroupId,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StopActionGroupRequest {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum StopActionGroupError {
    UnknownActionGroup(ActionGroupId),
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum HerderEvent {}
