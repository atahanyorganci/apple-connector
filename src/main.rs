use std::{
    env,
    error::Error,
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
};

use sqlx::{
    Connection,
    sqlite::{SqliteConnectOptions, SqliteConnection},
};

#[derive(Debug, sqlx::FromRow)]
struct MessageRow {
    row_id: i64,
    guid: String,
    text: Option<String>,
    date_utc: Option<String>,
    is_from_me: bool,
    sender: Option<String>,
}

impl MessageRow {
    fn direction(&self) -> &'static str {
        if self.is_from_me { "sent" } else { "received" }
    }

    fn sender_label(&self) -> &str {
        if self.is_from_me {
            "me"
        } else {
            self.sender.as_deref().unwrap_or("unknown sender")
        }
    }

    fn display_text(&self) -> &str {
        self.text
            .as_deref()
            .filter(|text| !text.is_empty())
            .unwrap_or("(empty text, may be in attributedBody)")
    }
}

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

    let messages = sqlx::query_as::<_, MessageRow>(
        r#"
        SELECT
            message.ROWID AS row_id,
            message.guid,
            message.text,
            datetime(
                message.date / 1000000000 + 978307200,
                'unixepoch'
            ) AS date_utc,
            message.is_from_me,
            handle.id AS sender
        FROM message
        LEFT JOIN handle ON message.handle_id = handle.ROWID
        WHERE message.item_type = 0
        ORDER BY message.date DESC
        LIMIT 5
        "#,
    )
    .fetch_all(&mut connection)
    .await?;

    for message in messages {
        println!(
            "{} | {} | {} | {} | {} | {}",
            message.row_id,
            message.date_utc.as_deref().unwrap_or("unknown date"),
            message.direction(),
            message.sender_label(),
            message.guid,
            message.display_text()
        );
    }

    Ok(())
}
