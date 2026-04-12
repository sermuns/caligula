use super::Herder;

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
    ) -> impl Future<Output = Result<super::data::SpawnActionGroupResponse, super::data::SpawnActionGroupError>> {
        todo!()
    }

    fn stop_action_group(
        &self,
        params: &super::data::StopActionGroupRequest,
    ) -> impl Future<Output = Result<super::data::StopActionGroupResponse, super::data::StopActionGroupError>> {
        todo!()
    }
}
