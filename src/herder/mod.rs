pub mod data;
mod fallback;
mod handle;
mod local;

pub use local::LocalHerder;

/// Handles the herding of action groups.
///
/// Why "Herder"? Caligula liked his horse, and horses are herded. I think. I'm not
/// a farmer.
pub struct HerderClient <W> {
}
#[tokio::main]
pub async fn remote_herder_main() {
    let h = LocalHerder::new();
}
