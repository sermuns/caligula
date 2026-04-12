use serde::{Deserialize, Serialize};

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
    IdTaken(ActionGroupId),
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StopActionGroupRequest {
    pub ag: ActionGroupId,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StopActionGroupResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum StopActionGroupError {
    UnknownActionGroup(ActionGroupId),
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum HerderEvent {}
