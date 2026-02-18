#![cfg_attr(test, allow(warnings))]

mod client;
mod client_store;
mod error;
mod service;
mod service_state;
mod session_stream;

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::{info, warn};

use base_client::audio_stream::AudioCapture;
use base_client::grpc_server::DictypeServer;
use config_tool::config_store::ConfigFile;
use pulseaudio_recorder::PulseAudioRecorder;

use crate::service::DictypeService;

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let runtime_dir = runtime_dir();
    fs::create_dir_all(&runtime_dir).expect("Create runtime dir for dictyped");
    let lock_path = runtime_dir.join("dictyped.lock");
    let _instance_lock = acquire_lock_file(&lock_path)?;

    let socket_path = runtime_dir.join("dictyped.socket");
    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
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
    let recorder = PulseAudioRecorder::new(config.pulseaudio().clone())?;
    let service = DictypeService::<PulseAudioRecorder>::new(
        client_store::ClientStore::load(&config),
        recorder,
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
fn runtime_dir() -> PathBuf {
    let uid = unsafe { libc::geteuid() };
    let mut path = PathBuf::from("/var/run/user");
    path.push(uid.to_string());
    path.push("dictype");
    path
}

#[cfg(unix)]
fn acquire_lock_file(lock_path: &Path) -> io::Result<File> {
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(lock_path)?;

    let result = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        return Ok(lock_file);
    }

    let err = io::Error::last_os_error();
    let raw_os_error = err.raw_os_error();
    if raw_os_error == Some(libc::EWOULDBLOCK) || raw_os_error == Some(libc::EAGAIN) {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "dictyped is already running (lock: {})",
                lock_path.display()
            ),
        ))
    } else {
        Err(io::Error::new(
            err.kind(),
            format!("failed to lock {}: {err}", lock_path.display()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn runtime_dir_test() {
        let runtime_dir = runtime_dir();
        assert_eq!(
            runtime_dir.file_name().and_then(|name| name.to_str()),
            Some("dictype")
        );
    }

    #[test]
    fn acquire_lock_file_rejects_second_holder() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before unix epoch")
            .as_nanos();
        let lock_path = std::env::temp_dir().join(format!(
            "dictyped-test-lock-{}-{}.lock",
            std::process::id(),
            nonce
        ));

        let first_guard =
            acquire_lock_file(&lock_path).expect("first lock acquisition should succeed");
        let second = acquire_lock_file(&lock_path);
        assert!(second.is_err(), "second lock acquisition should fail");
        assert_eq!(
            second.expect_err("second lock should return error").kind(),
            io::ErrorKind::AlreadyExists
        );

        drop(first_guard);
        acquire_lock_file(&lock_path)
            .expect("lock should be acquirable after first guard is dropped");

        let _ = fs::remove_file(lock_path);
    }
}
