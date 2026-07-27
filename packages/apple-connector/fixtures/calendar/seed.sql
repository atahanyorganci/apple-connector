-- Calendar fixture seed data for offline tests.
-- Timestamps use Core Data epoch (seconds since 2001-01-01 UTC).

INSERT INTO Store (ROWID, name, type, disabled, external_id, persistent_id, display_order)
VALUES (1, 'iCloud', 0, 0, 'store-icloud', 'persistent-icloud', 0);

INSERT INTO Calendar (ROWID, store_id, title, color, UUID, type, display_order)
VALUES (1, 1, 'Home', '#FF9500', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'local', 0);

INSERT INTO CalendarItem (
  ROWID, summary, description, start_date, end_date, all_day, calendar_id,
  status, invitation_status, availability, privacy_level, hidden,
  has_recurrences, has_attachment, has_attendees, UUID, entity_type,
  last_modified, creation_date, conference_url, travel_time
) VALUES (
  1,
  'Team Standup',
  'Daily sync with the engineering team',
  758635200.0,
  758637000.0,
  0,
  1,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  1,
  'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
  0,
  758635000.0,
  758630000.0,
  'https://meet.example.com/standup',
  900
);

INSERT INTO CalendarItem (
  ROWID, summary, start_date, end_date, all_day, calendar_id,
  hidden, has_recurrences, UUID, entity_type, last_modified
) VALUES (
  2,
  'Weekly Review',
  758635200.0,
  758637000.0,
  0,
  1,
  0,
  1,
  'cccccccc-cccc-cccc-cccc-cccccccccccc',
  0,
  758635000.0
);

INSERT INTO CalendarItem (
  ROWID, summary, start_date, end_date, all_day, calendar_id,
  hidden, UUID, entity_type, status, last_modified
) VALUES (
  3,
  'Hidden Planning',
  758640000.0,
  758641800.0,
  0,
  1,
  1,
  'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee',
  0,
  0,
  758635000.0
);

INSERT INTO Location (ROWID, title, address, latitude, longitude, item_owner_id)
VALUES (1, 'Conference Room A', '123 Main St, San Francisco, CA', 37774900, -122419400, 1);

UPDATE CalendarItem SET location_id = 1 WHERE ROWID = 1;

INSERT INTO Participant (ROWID, entity_type, type, status, role, owner_id, email, is_self, UUID)
VALUES
  (1, 0, 0, 1, 0, 1, 'alice@example.com', 0, '11111111-1111-1111-1111-111111111111'),
  (2, 0, 0, 1, 0, 1, 'bob@example.com', 1, '22222222-2222-2222-2222-222222222222');

INSERT INTO Recurrence (ROWID, frequency, interval, count, specifier, owner_id, UUID)
VALUES (1, 2, 1, 0, 'FREQ=WEEKLY;BYDAY=MO', 2, '33333333-3333-3333-3333-333333333333');

INSERT INTO ExceptionDate (ROWID, owner_id, date, sync_order)
VALUES (1, 2, 758721600.0, 0);

INSERT INTO Alarm (ROWID, trigger_interval, type, calendaritem_owner_id, UUID, disabled)
VALUES (1, -900, 0, 1, '44444444-4444-4444-4444-444444444444', 0);

INSERT INTO OccurrenceCache (
  day, event_id, calendar_id, store_id,
  occurrence_date, occurrence_start_date, occurrence_end_date
) VALUES (
  758635200.0,
  2,
  1,
  1,
  758635200.0,
  758635200.0,
  758637000.0
);

INSERT INTO AttachmentFile (ROWID, UUID, filename, format, local_path, file_size, store_id)
VALUES (1, 'dddddddd-dddd-dddd-dddd-dddddddddddd', 'agenda.pdf', 'com.adobe.pdf', 'Attachments/agenda.pdf', 1024, 1);

INSERT INTO Attachment (ROWID, owner_id, file_id)
VALUES (1, 1, 1);

INSERT INTO Conference (ROWID, uuid, owner_id, url, feature)
VALUES (1, '55555555-5555-5555-5555-555555555555', 1, 'https://meet.example.com/standup', 'video');
