use std::process::ExitCode;
use std::time::Duration;

use silence_cutter::api;
use silence_cutter::config::Config;
use silence_cutter::state;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "silence_cutter=info,tower_http=warn".into()),
        )
        .init();

    let config = Config::from_env();
    info!(
        addr = %config.addr,
        allowed_origins = ?config.allowed_origins,
        "starting silence-cutter backend"
    );
    tracing::debug!(?config, "full configuration");

    let listener = match TcpListener::bind(config.addr).await {
        Ok(listener) => listener,
        Err(err) => {
            let (ip, port) = (config.addr.ip(), config.addr.port());
            eprintln!(
                "error: could not bind http://{ip}:{port}: {err}\n\
                 hint: another instance may already be running, or the port is in use. \
                 Set SILENCE_CUTTER_ADDR=127.0.0.1:<other-port> to use a different port."
            );
            return ExitCode::FAILURE;
        }
    };

    // Sweep only after we own the port, and only files older than the TTL, so a
    // concurrently running instance's fresh source files are never deleted.
    let removed = state::sweep_stale_temp_files(Duration::from_secs(config.tmp_ttl_secs));
    if removed > 0 {
        info!(removed, "removed stale temp files from a previous run");
    }

    let app = api::router(config.clone());
    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    if let Err(err) = serve_result {
        tracing::error!(error = %err, "server error");
        return ExitCode::FAILURE;
    }

    info!("shutdown complete");
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("received shutdown signal");
}
