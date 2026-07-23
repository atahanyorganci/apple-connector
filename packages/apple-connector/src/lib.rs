mod api;
pub mod apple_types;
mod cli;
mod db;
pub mod fixtures;
mod messages;
mod notes;
mod reminders;

use std::{error::Error, io::Error as IoError, net::SocketAddr, path::PathBuf};

pub use api::{AppState, build_openapi_spec, router};
#[allow(deprecated)]
pub use cli::default_database_path;
pub use cli::{Cli, default_messages_database_path};
pub use db::{DatabaseError, connect_pool, database_open_failure, ensure_database_exists};
pub use messages::{
    AppBalloon, AppBalloonKind, Attachment, AttachmentBodyRef, AttachmentKind, AttachmentMessage,
    AttributedBodyDecodeError, AttributedRun, AudioMessage, BodyAttribute, Chat, Direction,
    GroupActionKind, GroupEvent, Handle, Message, MessageBody, MessageContent, MessageEnvelope,
    MessageInventory, PhotosBalloon, PollBalloon, PollOption, Reaction, ReactionAction,
    ReactionKind, ReplyRef, ReplyThread, ShareMyLocationMessage, ShareMyLocationStatus,
    SharePlayMessage, SystemMessage, Tapback, TextMessage, Transport, UnknownMessage, UrlBalloon,
    load_all, load_chats,
};
pub use notes::{NoteInventory, load_inventory as load_notes_inventory};
pub use reminders::{ReminderInventory, load_inventory};
use tracing::{info, warn};

pub async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    if cli.warns_about_public_binding() {
        warn!(
            "binding to 0.0.0.0 exposes an unauthenticated HTTP server on all interfaces; \
             place a reverse proxy or firewall in front before using this in production"
        );
    }

    let messages_path = cli.messages_database_path()?;
    ensure_database_exists(&messages_path)
        .map_err(|error| database_open_failure(&error, &messages_path))?;

    let messages_db = match connect_pool(&messages_path).await {
        Ok(pool) => Some(pool),
        Err(_error) => {
            warn!("Messages database could not be opened; API will report unavailable");
            None
        }
    };

    let reminders_path = resolve_reminders_path(&cli).await;
    let reminders_db = match reminders_path {
        Some(ref path) => match connect_pool(path).await {
            Ok(pool) => Some(pool),
            Err(_error) => {
                warn!("Reminders database could not be opened; API will report unavailable");
                None
            }
        },
        None => {
            warn!("Reminders database could not be discovered; API will report unavailable");
            None
        }
    };

    let notes_path = resolve_notes_path(&cli).await;
    let notes_db = match notes_path {
        Some(ref path) => match connect_pool(path).await {
            Ok(pool) => Some(pool),
            Err(_error) => {
                warn!("Notes database could not be opened; API will report unavailable");
                None
            }
        },
        None => {
            warn!("Notes database could not be resolved; API will report unavailable");
            None
        }
    };

    let attachment_root = cli.attachment_root_path()?;
    let reminders_attachment_root = resolve_reminders_attachment_root(&cli, &reminders_path);
    let notes_attachment_root = resolve_notes_attachment_root(&cli, &notes_path);
    let app = router(AppState::with_attachment_roots(
        messages_db,
        reminders_db,
        notes_db,
        attachment_root,
        reminders_attachment_root,
        notes_attachment_root,
    ));
    let address: SocketAddr = cli
        .socket_addr()
        .parse()
        .map_err(|error| IoError::other(format!("invalid socket address: {error}")))?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "apple-connector listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("apple-connector shut down");
    Ok(())
}

async fn resolve_reminders_path(cli: &Cli) -> Option<PathBuf> {
    if let Some(path) = &cli.reminders_database {
        if path.is_file() {
            return Some(path.clone());
        }
        warn!(path = %path.display(), "configured Reminders database path does not exist");
        return None;
    }

    if let Ok(path) = std::env::var("APPLE_CONNECTOR_REMINDERS_DATABASE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
        warn!(path = %path.display(), "APPLE_CONNECTOR_REMINDERS_DATABASE path does not exist");
        return None;
    }

    let stores_dir = cli.reminders_stores_dir_path().ok()?;
    match reminders::discovery::discover_reminders_database(&stores_dir).await {
        Ok(path) => Some(path),
        Err(error) => {
            warn!(error = %error.message(), "Reminders auto-discovery failed");
            None
        }
    }
}

fn resolve_reminders_attachment_root(cli: &Cli, reminders_path: &Option<PathBuf>) -> PathBuf {
    if let Some(path) = &cli.reminders_attachment_root {
        return path.clone();
    }

    reminders_path
        .as_ref()
        .map(|path| reminders::discovery::default_reminders_attachment_root(path))
        .unwrap_or_else(|| {
            PathBuf::from("/var/empty/apple-connector-reminders-attachments-unconfigured")
        })
}

async fn resolve_notes_path(cli: &Cli) -> Option<PathBuf> {
    let path = match cli.notes_database_path() {
        Ok(path) => path,
        Err(error) => {
            warn!(error = %error, "Notes database path could not be resolved");
            return None;
        }
    };

    if path.is_file() {
        return Some(path);
    }

    warn!(path = %path.display(), "configured Notes database path does not exist");
    None
}

fn resolve_notes_attachment_root(cli: &Cli, notes_path: &Option<PathBuf>) -> PathBuf {
    if let Some(path) = &cli.notes_attachment_root {
        return path.clone();
    }

    notes_path
        .as_ref()
        .and_then(|path| notes::discovery::notes_attachment_root_for_database(path).ok())
        .unwrap_or_else(|| {
            notes::discovery::default_notes_attachment_root().unwrap_or_else(|_| {
                PathBuf::from("/var/empty/apple-connector-notes-attachments-unconfigured")
            })
        })
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl-C handler");
    info!("shutdown signal received");
}
