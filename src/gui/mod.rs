mod app;

use app::App;
use std::sync::Arc;

use crate::{logging::LogPaths, orchestrator::Orchestrator, runtime::RemoteSpawn};

pub fn main(
    runtime: impl RemoteSpawn,
    orc: Arc<impl Orchestrator + Send + Sync + 'static>,
    log_paths: Arc<LogPaths>,
) -> eframe::Result<()> {
    eframe::run_native(
        "caligula-gui",
        Default::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc, runtime, orc, log_paths)))),
    )
}
