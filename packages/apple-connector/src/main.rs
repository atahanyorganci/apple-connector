use std::{
    env,
    error::Error,
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
};

use apple_connector::load_all;
use sqlx::{
    Connection,
    sqlite::{SqliteConnectOptions, SqliteConnection},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let home =
        env::var_os("HOME").ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    let database_path = PathBuf::from(home).join("Library/Messages/chat.db");

    if !database_path.is_file() {
        return Err(IoError::new(
            ErrorKind::NotFound,
            format!("Messages database not found at {}", database_path.display()),
        )
        .into());
    }

    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .read_only(true);
    let mut connection = SqliteConnection::connect_with(&options).await.map_err(|error| {
        IoError::new(
            ErrorKind::PermissionDenied,
            format!(
                "Could not open {}. Grant Full Disk Access to this terminal and try again: {error}",
                database_path.display()
            ),
        )
    })?;

    let messages = load_all(&mut connection).await?;

    for message in &messages {
        println!("{message:#?}\n");
    }

    eprintln!("loaded {} messages", messages.len());

    Ok(())
}
