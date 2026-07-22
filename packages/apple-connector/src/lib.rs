mod api;
mod cli;
mod db;
pub mod fixtures;
mod messages;

use std::{error::Error, io::Error as IoError, net::SocketAddr};

pub use api::{AppState, build_openapi_spec, router};
pub use cli::{Cli, default_database_path};
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
use tracing::{info, warn};

pub async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    if cli.warns_about_public_binding() {
        warn!(
            "binding to 0.0.0.0 exposes an unauthenticated HTTP server on all interfaces; \
             place a reverse proxy or firewall in front before using this in production"
        );
    }

    let database_path = cli.database_path()?;
    ensure_database_exists(&database_path)
        .map_err(|error| database_open_failure(&error, &database_path))?;

    let db = match connect_pool(&database_path).await {
        Ok(pool) => Some(pool),
        Err(error) => {
            warn!("{}", error.startup_message(&database_path));
            None
        }
    };

    let app = router(AppState::new(db));
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

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl-C handler");
    info!("shutdown signal received");
}
