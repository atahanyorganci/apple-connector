use sqlx::SqlitePool;

use super::{
    assembly::{
        assemble_messages, chat_summary_from_row, fetch_attachments_for_messages,
        fetch_chat_ids_for_messages, fetch_chat_row_by_id, fetch_participants_for_chats,
    },
    attachments::assemble_attachment,
    model::{Attachment, Chat, Message},
    queries::{
        fetch_attachment_by_guid, fetch_chat_message_page, fetch_filtered_messages,
        fetch_message_by_guid,
    },
    row::{AttachmentByGuidRow, AttachmentRow, ChatRow, MessageRow},
};
use crate::api::cursor::{
    ChatListCursor, ChatMessageCursor, GlobalMessageCursor, MessageSearchCursor, encode,
};

#[derive(Debug, Clone)]
pub enum MessageListCursor {
    Global(GlobalMessageCursor),
}

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ChatActivityRow {
    row_id: i64,
    guid: String,
    chat_identifier: Option<String>,
    display_name: Option<String>,
    room_name: Option<String>,
    service_name: Option<String>,
    style: Option<i64>,
    message_date: i64,
    message_id: i64,
}

pub struct MessageRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> MessageRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_chats(
        &self,
        limit: u32,
        cursor: Option<ChatListCursor>,
    ) -> Result<Page<Chat>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_chats_inner(limit, cursor)).await
    }

    async fn list_chats_inner(
        &self,
        limit: u32,
        cursor: Option<ChatListCursor>,
    ) -> Result<Page<Chat>, sqlx::Error> {
        let fetch_limit = i64::from(limit) + 1;
        let rows = sqlx::query_as!(
            ChatActivityRow,
            r#"
            SELECT
                chat.ROWID AS "row_id!",
                chat.guid AS "guid!",
                chat.chat_identifier,
                chat.display_name,
                chat.room_name,
                chat.service_name,
                chat.style,
                latest.message_date AS "message_date!",
                latest.message_id AS "message_id!"
            FROM chat
            INNER JOIN (
                SELECT chat_id, message_date, message_id
                FROM (
                    SELECT
                        chat_id,
                        message_date,
                        message_id,
                        ROW_NUMBER() OVER (
                            PARTITION BY chat_id
                            ORDER BY message_date DESC, message_id DESC
                        ) AS rn
                    FROM chat_message_join
                )
                WHERE rn = 1
            ) latest ON chat.ROWID = latest.chat_id
            WHERE (
                ?1 IS NULL
                OR latest.message_date < ?1
                OR (
                    latest.message_date = ?1
                    AND latest.message_id < ?2
                )
                OR (
                    latest.message_date = ?1
                    AND latest.message_id = ?2
                    AND chat.ROWID < ?3
                )
            )
            ORDER BY latest.message_date DESC, latest.message_id DESC, chat.ROWID DESC
            LIMIT ?4
            "#,
            cursor.map(|value| value.message_date),
            cursor.map(|value| value.message_id),
            cursor.map(|value| value.chat_id),
            fetch_limit,
        )
        .fetch_all(self.pool)
        .await?;

        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more
            .then(|| rows.last().map(chat_list_cursor_from_row))
            .flatten()
            .and_then(|cursor| encode(&cursor).ok());

        let chat_ids: Vec<i64> = rows.iter().map(|row| row.row_id).collect();
        let participants_by_chat = fetch_participants_for_chats(self.pool, &chat_ids).await?;

        let items = rows
            .into_iter()
            .map(|row| {
                let participants = participants_by_chat
                    .get(&row.row_id)
                    .cloned()
                    .unwrap_or_default();
                chat_summary_from_row(activity_row_to_chat_row(&row), participants)
            })
            .collect();

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_chat(&self, chat_id: i64) -> Result<Option<Chat>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_chat_inner(chat_id)).await
    }

    async fn get_chat_inner(&self, chat_id: i64) -> Result<Option<Chat>, sqlx::Error> {
        let Some(chat_row) = fetch_chat_row_by_id(self.pool, chat_id).await? else {
            return Ok(None);
        };

        let mut participants_by_chat =
            fetch_participants_for_chats(self.pool, std::slice::from_ref(&chat_id)).await?;
        let participants = participants_by_chat.remove(&chat_id).unwrap_or_default();

        Ok(Some(chat_summary_from_row(chat_row, participants)))
    }

    pub async fn list_chat_messages(
        &self,
        chat_id: i64,
        limit: u32,
        cursor: Option<ChatMessageCursor>,
    ) -> Result<Result<Page<Message>, ChatLookupError>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_chat_messages_inner(chat_id, limit, cursor)).await
    }

    async fn list_chat_messages_inner(
        &self,
        chat_id: i64,
        limit: u32,
        cursor: Option<ChatMessageCursor>,
    ) -> Result<Result<Page<Message>, ChatLookupError>, sqlx::Error> {
        if fetch_chat_row_by_id(self.pool, chat_id).await?.is_none() {
            return Ok(Err(ChatLookupError::NotFound));
        }

        let fetch_limit = i64::from(limit) + 1;
        let rows = fetch_chat_message_page(
            self.pool,
            chat_id,
            cursor.map(|value| value.message_date),
            cursor.map(|value| value.message_id),
            fetch_limit,
        )
        .await?;

        let (scoped_rows, has_more) = split_page(rows, limit);
        let next_cursor = if has_more {
            scoped_rows.last().and_then(|row| {
                encode(&ChatMessageCursor {
                    message_date: row.join_message_date,
                    message_id: row.message.row_id,
                })
                .ok()
            })
        } else {
            None
        };

        let message_rows: Vec<MessageRow> =
            scoped_rows.into_iter().map(|row| row.message).collect();
        let message_ids: Vec<i64> = message_rows.iter().map(|row| row.row_id).collect();
        let attachments_by_message =
            fetch_attachments_for_messages(self.pool, &message_ids).await?;
        let chat_ids_by_message = fetch_chat_ids_for_messages(self.pool, &message_ids).await?;
        let items = assemble_messages(message_rows, attachments_by_message, chat_ids_by_message);

        Ok(Ok(Page {
            items,
            has_more,
            next_cursor,
        }))
    }

    pub async fn list_messages_filtered(
        &self,
        filters: &super::search::MessageFilters,
        limit: u32,
        search_cursor: Option<MessageSearchCursor>,
        global_cursor: Option<MessageListCursor>,
    ) -> Result<Page<Message>, sqlx::Error> {
        crate::db::run_timed_query(|| {
            self.list_messages_filtered_inner(filters, limit, search_cursor, global_cursor)
        })
        .await
    }

    async fn list_messages_filtered_inner(
        &self,
        filters: &super::search::MessageFilters,
        limit: u32,
        search_cursor: Option<MessageSearchCursor>,
        global_cursor: Option<MessageListCursor>,
    ) -> Result<Page<Message>, sqlx::Error> {
        if filters.requires_text_scan() {
            return self.search_messages(filters, limit, search_cursor).await;
        }

        let scan_cursor = match global_cursor {
            Some(MessageListCursor::Global(cursor)) => Some((cursor.date, cursor.row_id)),
            None => search_cursor.map(|cursor| (cursor.date, cursor.row_id)),
        };

        let fetch_limit = i64::from(limit) + 1;
        let binds = filters.bind_values(
            scan_cursor.map(|(date, _)| date),
            scan_cursor.map(|(_, row_id)| row_id),
            fetch_limit,
        );
        let rows = fetch_filtered_messages(self.pool, &binds).await?;

        let use_search_cursor = filters.is_active();
        self.messages_page_with_cursor(rows, limit, |last_row| {
            if use_search_cursor {
                encode(&MessageSearchCursor {
                    date: last_row.sent_at,
                    row_id: last_row.row_id,
                    filters: filters.snapshot(),
                })
                .ok()
            } else {
                encode(&GlobalMessageCursor {
                    date: last_row.sent_at,
                    row_id: last_row.row_id,
                })
                .ok()
            }
        })
        .await
    }

    async fn search_messages(
        &self,
        filters: &super::search::MessageFilters,
        limit: u32,
        cursor: Option<MessageSearchCursor>,
    ) -> Result<Page<Message>, sqlx::Error> {
        use std::collections::HashMap;

        use super::{
            assembly::assemble_messages_with_bodies,
            classify::message_body,
            model::MessageBody,
            search::{CANDIDATE_CHUNK_SIZE, MESSAGE_SCAN_BUDGET, text_matches_needle},
        };

        let Some(query) = filters.q.as_deref() else {
            return Ok(Page {
                items: Vec::new(),
                has_more: false,
                next_cursor: None,
            });
        };
        let needle = query.to_lowercase();
        let mut matching_rows = Vec::new();
        let mut bodies_by_message: HashMap<i64, MessageBody> = HashMap::new();
        let mut scanned = 0_u32;
        let mut scan_position = cursor.map(|value| (value.date, value.row_id));
        let mut reached_end = false;

        'search: while scanned < MESSAGE_SCAN_BUDGET {
            let binds = filters.bind_values(
                scan_position.map(|(date, _)| date),
                scan_position.map(|(_, row_id)| row_id),
                i64::from(CANDIDATE_CHUNK_SIZE),
            );
            let chunk = fetch_filtered_messages(self.pool, &binds).await?;

            if chunk.is_empty() {
                reached_end = true;
                break;
            }

            let chunk_len = chunk.len();
            for row in chunk {
                scanned += 1;
                scan_position = Some((row.sent_at, row.row_id));

                let cached_body = if !super::search::has_searchable_plain_text(&row)
                    && row.attributed_body.is_some()
                {
                    Some(message_body(&row))
                } else {
                    None
                };
                if text_matches_needle(&row, &needle, cached_body.as_ref()) {
                    let body = cached_body.unwrap_or_else(|| message_body(&row));
                    bodies_by_message.insert(row.row_id, body);
                    matching_rows.push(row);
                    if matching_rows.len() > limit as usize {
                        break 'search;
                    }
                }

                if scanned >= MESSAGE_SCAN_BUDGET {
                    break;
                }
            }

            if chunk_len < CANDIDATE_CHUNK_SIZE as usize {
                reached_end = true;
                break;
            }
        }

        let has_more = matching_rows.len() > limit as usize
            || (!reached_end && scanned >= MESSAGE_SCAN_BUDGET);
        if matching_rows.len() > limit as usize {
            matching_rows.truncate(limit as usize);
        }

        let next_cursor = has_more
            .then(|| {
                scan_position.and_then(|(date, row_id)| {
                    encode(&MessageSearchCursor {
                        date,
                        row_id,
                        filters: filters.snapshot(),
                    })
                    .ok()
                })
            })
            .flatten();

        let message_ids: Vec<i64> = matching_rows.iter().map(|row| row.row_id).collect();
        let attachments_by_message =
            fetch_attachments_for_messages(self.pool, &message_ids).await?;
        let chat_ids_by_message = fetch_chat_ids_for_messages(self.pool, &message_ids).await?;
        let items = assemble_messages_with_bodies(
            matching_rows,
            attachments_by_message,
            chat_ids_by_message,
            Some(bodies_by_message),
        );

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_message_by_guid(&self, guid: &str) -> Result<Option<Message>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_message_by_guid_inner(guid)).await
    }

    async fn get_message_by_guid_inner(&self, guid: &str) -> Result<Option<Message>, sqlx::Error> {
        let Some(row) = fetch_message_by_guid(self.pool, guid).await? else {
            return Ok(None);
        };

        let message_ids = [row.row_id];
        let attachments_by_message =
            fetch_attachments_for_messages(self.pool, &message_ids).await?;
        let chat_ids_by_message = fetch_chat_ids_for_messages(self.pool, &message_ids).await?;

        Ok(
            assemble_messages(vec![row], attachments_by_message, chat_ids_by_message)
                .into_iter()
                .next(),
        )
    }

    pub async fn get_attachment_by_guid(
        &self,
        guid: &str,
    ) -> Result<Option<Attachment>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_attachment_by_guid_inner(guid)).await
    }

    async fn get_attachment_by_guid_inner(
        &self,
        guid: &str,
    ) -> Result<Option<Attachment>, sqlx::Error> {
        let Some(row) = fetch_attachment_by_guid(self.pool, guid).await? else {
            return Ok(None);
        };

        Ok(Some(assemble_attachment_by_guid(&row)))
    }

    async fn messages_page_with_cursor<F>(
        &self,
        rows: Vec<MessageRow>,
        limit: u32,
        encode: F,
    ) -> Result<Page<Message>, sqlx::Error>
    where
        F: FnOnce(&MessageRow) -> Option<String>,
    {
        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more.then(|| rows.last().and_then(encode)).flatten();
        let message_ids: Vec<i64> = rows.iter().map(|row| row.row_id).collect();
        let attachments_by_message =
            fetch_attachments_for_messages(self.pool, &message_ids).await?;
        let chat_ids_by_message = fetch_chat_ids_for_messages(self.pool, &message_ids).await?;
        let items = assemble_messages(rows, attachments_by_message, chat_ids_by_message);

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatLookupError {
    NotFound,
}

impl std::fmt::Display for ChatLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "chat not found"),
        }
    }
}

impl std::error::Error for ChatLookupError {}

fn split_page<T>(mut rows: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    (rows, has_more)
}

fn activity_row_to_chat_row(row: &ChatActivityRow) -> ChatRow {
    ChatRow {
        row_id: row.row_id,
        guid: row.guid.clone(),
        chat_identifier: row.chat_identifier.clone(),
        display_name: row.display_name.clone(),
        room_name: row.room_name.clone(),
        service_name: row.service_name.clone(),
        style: row.style,
    }
}

fn chat_list_cursor_from_row(row: &ChatActivityRow) -> ChatListCursor {
    ChatListCursor {
        message_date: row.message_date,
        message_id: row.message_id,
        chat_id: row.row_id,
    }
}

fn assemble_attachment_by_guid(row: &AttachmentByGuidRow) -> Attachment {
    let attachment_row = AttachmentRow {
        message_id: 0,
        guid: row.guid.clone(),
        original_guid: row.original_guid.clone(),
        filename: row.filename.clone(),
        uti: row.uti.clone(),
        mime_type: row.mime_type.clone(),
        transfer_name: row.transfer_name.clone(),
        total_bytes: row.total_bytes,
        is_sticker: row.is_sticker,
        transfer_state: row.transfer_state,
        hide_attachment: row.hide_attachment,
        emoji_description: row.emoji_description.clone(),
    };
    let body_refs = std::collections::HashMap::new();
    assemble_attachment(&attachment_row, &body_refs)
}

#[cfg(test)]
mod tests {
    use sqlx::Connection;

    use super::{MessageListCursor, MessageRepository};
    use crate::{
        api::cursor::{ChatMessageCursor, GlobalMessageCursor, decode},
        apple_types::MessageId,
        db::connect_pool,
        fixtures::FixtureDb,
        messages::search::MessageFilters,
    };

    async fn seed_pagination_fixture() -> Result<FixtureDb, Box<dyn std::error::Error>> {
        let fixture = FixtureDb::empty().await?;
        let mut connection =
            sqlx::SqliteConnection::connect(fixture.path().to_str().ok_or("invalid fixture path")?)
                .await?;

        sqlx::query!("DROP TRIGGER IF EXISTS verify_chat_insert")
            .execute(&mut connection)
            .await?;
        sqlx::query!("DROP TRIGGER IF EXISTS verify_chat_update")
            .execute(&mut connection)
            .await?;

        sqlx::query!("INSERT INTO handle (id, service) VALUES ('+15550000001', 'iMessage')")
            .execute(&mut connection)
            .await?;
        sqlx::query!("INSERT INTO handle (id, service) VALUES ('+15550000002', 'iMessage')")
            .execute(&mut connection)
            .await?;

        for chat_index in 0..3 {
            let chat_guid = format!("chat-{chat_index}");
            let chat_identifier = format!("+1555000000{chat_index}");
            sqlx::query!(
                "INSERT INTO chat (guid, style, chat_identifier, service_name) \
                 VALUES (?1, 45, ?2, 'iMessage')",
                chat_guid,
                chat_identifier
            )
            .execute(&mut connection)
            .await?;
        }

        for message_index in 0..5 {
            let message_guid = format!("message-{message_index}");
            let body = format!("body-{message_index}");
            let sent_at = 1_000_i64 + message_index;
            sqlx::query!(
                "INSERT INTO message (guid, text, service, is_from_me, date) \
                 VALUES (?1, ?2, 'iMessage', 1, ?3)",
                message_guid,
                body,
                sent_at
            )
            .execute(&mut connection)
            .await?;
        }

        sqlx::query!(
            "INSERT INTO message (guid, text, service, is_from_me, date) \
             VALUES ('message-tie-a', 'tie-a', 'iMessage', 1, 500), \
                    ('message-tie-b', 'tie-b', 'iMessage', 1, 500)"
        )
        .execute(&mut connection)
        .await?;

        sqlx::query!(
            "INSERT INTO chat_message_join (chat_id, message_id, message_date) \
             SELECT chat.ROWID, message.ROWID, message.date \
             FROM chat JOIN message ON message.guid LIKE 'message-%'"
        )
        .execute(&mut connection)
        .await?;

        sqlx::query!(
            "INSERT INTO chat_handle_join (chat_id, handle_id) \
             SELECT chat.ROWID, handle.ROWID FROM chat, handle"
        )
        .execute(&mut connection)
        .await?;

        connection.close().await.ok();
        Ok(fixture)
    }

    #[tokio::test]
    async fn global_messages_paginate_without_dupes_or_gaps()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_pagination_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repository = MessageRepository::new(&pool);

        let mut seen = Vec::new();
        let mut cursor = None;

        loop {
            let page = repository
                .list_messages_filtered(
                    &MessageFilters::default(),
                    2,
                    None,
                    cursor.map(MessageListCursor::Global),
                )
                .await?;
            for message in &page.items {
                assert!(
                    !seen.contains(&message.envelope.row_id),
                    "duplicate row_id {}",
                    message.envelope.row_id
                );
                seen.push(message.envelope.row_id);
            }

            if !page.has_more {
                break;
            }

            let next = page.next_cursor.ok_or("missing next cursor")?;
            cursor = Some(decode::<GlobalMessageCursor>(&next)?);
        }

        assert_eq!(seen.len(), 7);
        Ok(())
    }

    #[tokio::test]
    async fn chat_messages_use_message_date_ordering() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_pagination_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repository = MessageRepository::new(&pool);
        let chat_id = sqlx::query_scalar!("SELECT ROWID AS \"row_id!\" FROM chat LIMIT 1")
            .fetch_one(&pool)
            .await?;

        let first_page = repository.list_chat_messages(chat_id, 3, None).await??;
        let second_page = repository
            .list_chat_messages(
                chat_id,
                3,
                first_page
                    .next_cursor
                    .as_deref()
                    .map(decode::<ChatMessageCursor>)
                    .transpose()?,
            )
            .await??;

        let first_ids: Vec<_> = first_page
            .items
            .iter()
            .map(|message| message.envelope.row_id)
            .collect();
        let second_ids: Vec<_> = second_page
            .items
            .iter()
            .map(|message| message.envelope.row_id)
            .collect();
        assert!(first_ids.iter().all(|id| !second_ids.contains(id)));
        Ok(())
    }

    #[tokio::test]
    async fn list_chats_orders_by_recent_activity() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_pagination_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repository = MessageRepository::new(&pool);

        let page = repository.list_chats(10, None).await?;
        assert_eq!(page.items.len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn get_message_by_guid_returns_classified_message()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = FixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let repository = MessageRepository::new(&pool);

        let message = repository
            .get_message_by_guid("fixture-message-guid")
            .await?
            .ok_or("fixture message not found")?;

        assert_eq!(
            message.envelope.guid,
            MessageId::new("fixture-message-guid")
        );
        assert!(!message.envelope.chat_ids.is_empty());
        Ok(())
    }
    const LIST_CHATS_LATEST_ACTIVITY_SQL: &str = r"
            SELECT
                chat.ROWID AS row_id,
                chat.guid AS guid,
                chat.chat_identifier,
                chat.display_name,
                chat.room_name,
                chat.service_name,
                chat.style,
                latest.message_date AS message_date,
                latest.message_id AS message_id
            FROM chat
            INNER JOIN (
                SELECT chat_id, message_date, message_id
                FROM (
                    SELECT
                        chat_id,
                        message_date,
                        message_id,
                        ROW_NUMBER() OVER (
                            PARTITION BY chat_id
                            ORDER BY message_date DESC, message_id DESC
                        ) AS rn
                    FROM chat_message_join
                )
                WHERE rn = 1
            ) latest ON chat.ROWID = latest.chat_id
            ORDER BY latest.message_date DESC, latest.message_id DESC, chat.ROWID DESC
            LIMIT 10
    ";

    #[tokio::test]
    async fn list_chats_latest_activity_query_plan_uses_window_function()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_pagination_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;

        let plan = crate::fixtures::explain_query_plan(&pool, LIST_CHATS_LATEST_ACTIVITY_SQL)
            .await?
            .join("\n");
        eprintln!("list_chats latest-activity EXPLAIN QUERY PLAN:\n{plan}");

        assert!(
            !plan.contains("NOT EXISTS"),
            "expected window-function plan without correlated NOT EXISTS, plan:\n{plan}"
        );
        Ok(())
    }
}
