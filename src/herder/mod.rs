mod handle;
mod remote;
mod local;
pub mod ipc;
mod fallback;

use crossterm::event::EventStream;
use futures::Stream;


/// Handles the herding of action groups.
///
/// Why "Herder"? Caligula liked his horse, and horses are herded. I think. I'm not
/// a farmer.
pub trait Herder {
    /// Spawn a new action group.
    fn spawn_action_group(&self, params: &ipc::SpawnActionGroupRequest) -> impl Future<Output = Result<ipc::SpawnActionGroupResponse, ipc::SpawnActionGroupError>>;

    /// Stop a running action group.
    fn stop_action_group(&self, params: &ipc::StopActionGroupRequest) -> impl Future<Output=Result<ipc::StopActionGroupResponse, ipc::StopActionGroupError>>;
}
