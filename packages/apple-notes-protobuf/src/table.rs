//! Tables embedded in notes.
//!
//! A table is not part of the note body: the body holds a U+FFFC placeholder and an attachment
//! reference, and the attachment row keeps the table in its own gzip-compressed
//! `MergableDataProto` (`ZMERGEABLEDATA1`). That document is a CRDT whose entries refer to each
//! other by index:
//!
//! - the root entry is a map of type `com.apple.notes.ICTable` whose keys name `crRows`,
//!   `crColumns`, `cellColumns`, and `crTableColumnDirection`;
//! - `crRows` and `crColumns` are ordered sets of UUIDs, giving each row and column its position;
//! - `cellColumns` is a dictionary from column UUID to a dictionary from row UUID to the cell, a
//!   `Note` message like the body's own;
//! - `crTableColumnDirection` points, through a register, at the direction string.
//!
//! Field numbers follow `proto/notestore.proto` in apple_cloud_notes_parser, which the
//! `fixtures/notes/bodies/acnp` table fixtures come from.

use std::collections::HashMap;

use crate::{
    DecodeError, Limits, decompress_gzip,
    protobuf::{Budget, Field, all_bytes, fields_by_number, first_bytes, parse_message},
};

const TABLE_TYPE: &str = "com.apple.notes.ICTable";
const RIGHT_TO_LEFT: &str = "CRTableColumnDirectionRightToLeft";

/// A table reconstructed from an attachment's mergeable data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteTable {
    /// Cell text, row by row. Columns are in visual order, left to right, so a right-to-left
    /// table's first column comes last.
    pub rows: Vec<Vec<String>>,
    /// The table is laid out right to left.
    pub right_to_left: bool,
}

/// Decode a table attachment's gzip-compressed mergeable data, using the default [`Limits`].
pub fn decode_table(data: &[u8]) -> Result<NoteTable, DecodeError> {
    decode_table_with_limits(data, &Limits::default())
}

/// Decode a table attachment's gzip-compressed mergeable data under caller-supplied budgets.
pub fn decode_table_with_limits(data: &[u8], limits: &Limits) -> Result<NoteTable, DecodeError> {
    if data.is_empty() {
        return Err(DecodeError::Empty);
    }
    let document = decompress_gzip(data, limits)?;
    let mut budget = Budget::new(limits.max_fields, limits.max_message_depth);

    let root = parse_message(&document, &mut budget, 0)?;
    let object = parse_message(
        required(first_bytes(&root, 2), "mergeable data object")?,
        &mut budget,
        1,
    )?;
    let data = parse_message(
        required(first_bytes(&object, 3), "mergeable data")?,
        &mut budget,
        2,
    )?;
    let mut entries = Vec::new();
    for entry in all_bytes(&data, 3) {
        entries.push(parse_message(entry, &mut budget, 3)?);
    }
    let doc = Document {
        entries,
        keys: strings(all_bytes(&data, 4)),
        types: strings(all_bytes(&data, 5)),
        uuids: all_bytes(&data, 6),
    };
    doc.table(&mut budget, limits)
}

fn invalid(message: &str) -> DecodeError {
    DecodeError::InvalidProtobuf(format!("table: {message}"))
}

fn required<T>(value: Option<T>, what: &str) -> Result<T, DecodeError> {
    value.ok_or_else(|| invalid(&format!("missing {what}")))
}

/// Non-UTF-8 items become empty strings, which match no known key or type.
fn strings(items: Vec<&[u8]>) -> Vec<&str> {
    items
        .into_iter()
        .map(|item| std::str::from_utf8(item).unwrap_or_default())
        .collect()
}

/// An `ObjectID`: a number, a string, or a reference to another entry.
struct ObjectId<'a> {
    unsigned: Option<u64>,
    string: Option<&'a str>,
    index: Option<usize>,
}

fn object_id<'a>(
    bytes: &'a [u8],
    budget: &mut Budget,
    depth: usize,
) -> Result<ObjectId<'a>, DecodeError> {
    let fields = parse_message(bytes, budget, depth)?;
    Ok(ObjectId {
        unsigned: fields_by_number(&fields, 2).find_map(|field| field.varint),
        string: first_bytes(&fields, 4).and_then(|bytes| std::str::from_utf8(bytes).ok()),
        index: fields_by_number(&fields, 6)
            .find_map(|field| field.varint)
            .and_then(|value| usize::try_from(value).ok()),
    })
}

struct Document<'a> {
    entries: Vec<Vec<Field<'a>>>,
    keys: Vec<&'a str>,
    types: Vec<&'a str>,
    uuids: Vec<&'a [u8]>,
}

impl<'a> Document<'a> {
    fn table(&self, budget: &mut Budget, limits: &Limits) -> Result<NoteTable, DecodeError> {
        let table_type = self
            .types
            .iter()
            .position(|kind| *kind == TABLE_TYPE)
            .and_then(|position| u64::try_from(position).ok())
            .ok_or_else(|| invalid("no ICTable type"))?;

        let mut root = None;
        for entry in &self.entries {
            if let Some((kind, pairs)) = self.custom_map(entry, budget)?
                && kind == table_type
            {
                root = Some(pairs);
                break;
            }
        }
        let root = root.ok_or_else(|| invalid("no ICTable entry"))?;

        let (mut rows, mut columns, mut cells, mut direction) = (None, None, None, None);
        for (key, value) in &root {
            let name = usize::try_from(*key)
                .ok()
                .and_then(|key| self.keys.get(key))
                .copied();
            match name {
                Some("crRows") => rows = Some(value.index),
                Some("crColumns") => columns = Some(value.index),
                Some("cellColumns") => cells = Some(value.index),
                Some("crTableColumnDirection") => direction = Some(value.index),
                _ => {}
            }
        }

        let (row_count, row_positions) = self.ordering(required(rows, "crRows")?, budget)?;
        let (column_count, column_positions) =
            self.ordering(required(columns, "crColumns")?, budget)?;
        let cell_count = row_count.saturating_mul(column_count);
        if cell_count > limits.max_table_cells {
            return Err(DecodeError::LimitExceeded {
                limit: "table cell count",
                actual: cell_count,
                max: limits.max_table_cells,
            });
        }

        let mut table = vec![vec![String::new(); column_count]; row_count];
        if let Some(cells) = cells {
            let columns_dictionary = required(first_bytes(self.entry(cells)?, 6), "cellColumns")?;
            for (column_key, column_value) in self.dictionary(columns_dictionary, budget, 4)? {
                let column = self.uuid_index(column_key.index, budget)?;
                let Some(rows_dictionary) = first_bytes(self.entry(column_value.index)?, 6) else {
                    continue;
                };
                for (row_key, row_value) in self.dictionary(rows_dictionary, budget, 4)? {
                    let row = self.uuid_index(row_key.index, budget)?;
                    let (Some(row), Some(column)) =
                        (row_positions.get(&row), column_positions.get(&column))
                    else {
                        continue;
                    };
                    let text = self.cell_text(row_value.index, budget)?;
                    if let Some(cell) = table.get_mut(*row).and_then(|row| row.get_mut(*column)) {
                        *cell = text;
                    }
                }
            }
        }

        let right_to_left = match direction {
            Some(index) => self.right_to_left(index, budget)?,
            None => false,
        };
        if right_to_left {
            for row in &mut table {
                row.reverse();
            }
        }
        Ok(NoteTable {
            rows: table,
            right_to_left,
        })
    }

    fn entry(&self, index: Option<usize>) -> Result<&[Field<'a>], DecodeError> {
        index
            .and_then(|index| self.entries.get(index))
            .map(Vec::as_slice)
            .ok_or_else(|| invalid("object index out of range"))
    }

    /// An entry's custom map (field 13): its type index and `(key index, value)` pairs.
    #[allow(clippy::type_complexity)]
    fn custom_map(
        &self,
        entry: &[Field<'a>],
        budget: &mut Budget,
    ) -> Result<Option<(u64, Vec<(u64, ObjectId<'a>)>)>, DecodeError> {
        let Some(bytes) = first_bytes(entry, 13) else {
            return Ok(None);
        };
        let map = parse_message(bytes, budget, 4)?;
        let kind = required(
            fields_by_number(&map, 1).find_map(|field| field.varint),
            "map type",
        )?;
        let mut pairs = Vec::new();
        for map_entry in all_bytes(&map, 3) {
            let fields = parse_message(map_entry, budget, 5)?;
            let key = fields_by_number(&fields, 1).find_map(|field| field.varint);
            if let (Some(key), Some(value)) = (key, first_bytes(&fields, 2)) {
                pairs.push((key, object_id(value, budget, 6)?));
            }
        }
        Ok(Some((kind, pairs)))
    }

    /// The UUID index an `NSUUID` entry holds.
    fn uuid_index(&self, index: Option<usize>, budget: &mut Budget) -> Result<u64, DecodeError> {
        let (_, pairs) = required(self.custom_map(self.entry(index)?, budget)?, "UUID map")?;
        required(
            pairs.first().and_then(|(_, value)| value.unsigned),
            "UUID index",
        )
    }

    /// `(key, value)` pairs of a `Dictionary` message.
    fn dictionary(
        &self,
        bytes: &'a [u8],
        budget: &mut Budget,
        depth: usize,
    ) -> Result<Vec<(ObjectId<'a>, ObjectId<'a>)>, DecodeError> {
        let dictionary = parse_message(bytes, budget, depth)?;
        let mut pairs = Vec::new();
        for element in all_bytes(&dictionary, 1) {
            let fields = parse_message(element, budget, depth + 1)?;
            let key = object_id(
                required(first_bytes(&fields, 1), "dictionary key")?,
                budget,
                depth + 2,
            )?;
            let value = object_id(
                required(first_bytes(&fields, 2), "dictionary value")?,
                budget,
                depth + 2,
            )?;
            pairs.push((key, value));
        }
        Ok(pairs)
    }

    /// Positions from an ordered set (field 16): how many rows or columns it has, and the
    /// position of each UUID index. Every UUID an ordering element aliases gets the position of
    /// the UUID it aliases.
    fn ordering(
        &self,
        index: Option<usize>,
        budget: &mut Budget,
    ) -> Result<(usize, HashMap<u64, usize>), DecodeError> {
        let set = parse_message(
            required(first_bytes(self.entry(index)?, 16), "ordered set")?,
            budget,
            4,
        )?;
        let ordering = parse_message(required(first_bytes(&set, 1), "ordering")?, budget, 5)?;
        let array = parse_message(
            required(first_bytes(&ordering, 1), "ordering array")?,
            budget,
            6,
        )?;

        let mut positions = HashMap::new();
        let mut count = 0_usize;
        for attachment in all_bytes(&array, 2) {
            let fields = parse_message(attachment, budget, 7)?;
            let uuid = required(first_bytes(&fields, 2), "ordering UUID")?;
            if let Some(position) = self
                .uuids
                .iter()
                .position(|candidate| *candidate == uuid)
                .and_then(|position| u64::try_from(position).ok())
            {
                positions.insert(position, count);
            }
            count += 1;
        }

        if let Some(contents) = first_bytes(&ordering, 2) {
            for (key, value) in self.dictionary(contents, budget, 6)? {
                let from = self.uuid_index(key.index, budget)?;
                let to = self.uuid_index(value.index, budget)?;
                if let Some(position) = positions.get(&from).copied() {
                    positions.insert(to, position);
                }
            }
        }
        Ok((count, positions))
    }

    /// The text of a cell's `Note` (field 10); empty when the cell has none.
    fn cell_text(&self, index: Option<usize>, budget: &mut Budget) -> Result<String, DecodeError> {
        let Some(note) = first_bytes(self.entry(index)?, 10) else {
            return Ok(String::new());
        };
        let fields = parse_message(note, budget, 4)?;
        Ok(first_bytes(&fields, 2)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or_default()
            .to_owned())
    }

    /// `crTableColumnDirection` points at a register whose contents name an `NSString` entry.
    fn right_to_left(
        &self,
        index: Option<usize>,
        budget: &mut Budget,
    ) -> Result<bool, DecodeError> {
        let Some(register) = first_bytes(self.entry(index)?, 1) else {
            return Ok(false);
        };
        let register = parse_message(register, budget, 4)?;
        let Some(contents) = first_bytes(&register, 2) else {
            return Ok(false);
        };
        let target = object_id(contents, budget, 5)?;
        let Some((_, pairs)) = self.custom_map(self.entry(target.index)?, budget)? else {
            return Ok(false);
        };
        Ok(pairs
            .iter()
            .any(|(_, value)| value.string == Some(RIGHT_TO_LEFT)))
    }
}
