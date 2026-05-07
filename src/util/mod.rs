use std::{
    env, fs::DirBuilder, os::unix::fs::DirBuilderExt as _, path::PathBuf, process, time::SystemTime,
};

pub mod candidate;

/// Create the directory to shove invocation-specific data into, like log files
/// and sockets.
pub fn ensure_state_dir() -> std::io::Result<PathBuf> {
    let dir = env::temp_dir().join(format!(
        "caligula-{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    ));

    DirBuilder::new().mode(0o700).recursive(true).create(&dir)?;

    Ok(dir)
}
