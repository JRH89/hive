//! `hived` — normally started on demand by `hive`, not by hand.

use std::fs::OpenOptions;
use std::process::ExitCode;

use hive_core::config::Config;
use hive_core::paths::Paths;
use hive_daemon::AlreadyRunning;

fn main() -> ExitCode {
    // Leave the launching terminal's session so closing it (or the frontend)
    // never delivers SIGHUP to us. Fails harmlessly if already a leader.
    #[cfg(unix)]
    let _ = rustix::process::setsid();

    let paths = Paths::resolve();
    if let Err(e) = init_logging(&paths) {
        eprintln!("hived: cannot open log: {e}");
    }

    let config = match Config::load(&paths.config_file) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{e}");
            eprintln!("hived: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    match runtime.block_on(hive_daemon::run(paths, config)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.is::<AlreadyRunning>() => {
            tracing::debug!("{e}; exiting");
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!("{e:#}");
            eprintln!("hived: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging(paths: &Paths) -> std::io::Result<()> {
    paths.ensure_state_dir()?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.state_dir.join("hived.log"))?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("HIVE_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .init();
    Ok(())
}
