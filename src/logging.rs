use std::fs::File;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use tracing::Level;

#[cfg(not(debug_assertions))]
const FILE_LOG_LEVEL: Level = Level::DEBUG;

#[cfg(debug_assertions)]
const FILE_LOG_LEVEL: Level = Level::TRACE;

macro_rules! base_tracing_subscriber {
    () => {
        ::tracing_subscriber::fmt()
            .compact()
            .with_ansi(false)
            .with_env_filter(
                ::tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(crate::logging::FILE_LOG_LEVEL.into())
                    .from_env_lossy(),
            )
            .with_file(true)
            .with_line_number(true)
    };
}

#[derive(Clone)]
pub struct LogFile {
    file: Option<Arc<std::sync::Mutex<File>>>,
}

impl std::io::Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let Some(f) = &self.file else {
            return Ok(buf.len());
        };
        f.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let Some(f) = &self.file else {
            return Ok(());
        };
        f.lock().unwrap().flush()
    }
}

/// Set up logging configuration for the main parent process.
///
/// Returns the [LogFile], which is appropriate for writing child process logs to.
pub fn setup_parent_logging() -> LogFile {
    let path = format!(
        "/tmp/caligula-{}.log",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
    );

    let file = match File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("WARNING: Failed to create log file at {path}: {e}");
            eprintln!("If you encounter bugs, you're on your own!");
            return LogFile { file: None };
        }
    };
    let file = LogFile {
        file: Some(Arc::new(std::sync::Mutex::new(file))),
    };

    // Print out the log file if we're in debug mode
    #[cfg(debug_assertions)]
    eprintln!("Log file is at {path}");

    // Initialize tracing subscriber
    let cloned = file.clone();
    base_tracing_subscriber!()
        .with_writer(move || cloned.clone())
        .init();

    // Set up panic hook to give users a message if things go haywire
    std::panic::set_hook(Box::new(move |p| {
        tracing_panic::panic_hook(p);

        crossterm::terminal::disable_raw_mode().ok();

        eprintln!("An unexpected error occurred: {p}");
        eprintln!();
        eprintln!(
            "Please report bugs to https://github.com/ifd3f/caligula/issues and attach the log file at {path}",
        );
    }));

    file
}

/// Set up logging configuration suitable for a child process.
pub fn setup_child_logging() {
    // Log things to stderr
    base_tracing_subscriber!()
        .with_writer(std::io::stderr)
        .init();

    // Set up a panic hook for reporting panics
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));
}
