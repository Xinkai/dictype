#![cfg_attr(test, allow(warnings))]

mod client;
mod client_store;
mod error;
mod service;
mod service_state;
mod session_stream;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::PathBuf};

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::{info, warn};

use base_client::audio_stream::AudioStream;
use base_client::grpc_server::DictypeServer;
use config_tool::config_store::ConfigFile;
use pulseaudio_recorder::PulseAudioRecorder;

use crate::service::DictypeService;

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let socket_path = socket_path();

    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
    } else if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent).expect("Create parent dir for socket");
    }

    let listener = UnixListener::bind(&socket_path)?;
    let permissions = fs::Permissions::from_mode(0o600);
    if let Err(err) = fs::set_permissions(&socket_path, permissions) {
        warn!("failed to adjust socket permissions: {err}");
    }

    let config = ConfigFile::load().unwrap_or_else(|err| {
        warn!("failed to load config, using defaults: {err}");
        ConfigFile::default()
    });
    let service = DictypeService::new(
        client_store::ClientStore::load(&config),
        PulseAudioRecorder::new,
        config.pulseaudio().clone(),
    );
    let incoming = UnixListenerStream::new(listener);

    info!("listening on {}", socket_path.display());

    Ok(Server::builder()
        .add_service(DictypeServer::new(service))
        .serve_with_incoming(incoming)
        .await?)
}

fn init_tracing() {
    // Use env filter when configured, fall back to info-level console logging.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();
}

#[cfg(unix)]
fn socket_path() -> PathBuf {
    let uid = unsafe { libc::geteuid() };
    let mut path = PathBuf::from("/var/run/user");
    path.push(uid.to_string());
    path.push("dictype");
    path.push("dictyped.socket");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_test() {
        let _ = socket_path();
    }
}
