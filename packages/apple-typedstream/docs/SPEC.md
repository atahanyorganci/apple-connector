# `apple-typedstream`: reader and writer for NeXT/Apple typedstream archives

Parses and writes the binary `NSArchiver` typedstream format. `apple-connector` uses it to read
Messages `attributedBody` blobs from `chat.db`. It is a real Serde format: values can be read into a
dynamic `Value` tree or into any `Deserialize` type.

## Kind

Codec.

## Public API

- `value_from_slice(&[u8]) -> Result<Value>` and `value_from_reader(R) -> Result<Value>`: the
  faithful view, keeping archived objects, classes, and back-references.
- `from_slice<T: DeserializeOwned>`, `from_reader<R, T>`, `from_reader_with_limit<R, T>(reader, limit)`:
  Serde deserialization; archived objects are flattened into plain maps.
- `to_vec<T: Serialize>`, `to_writer<W, T>`: Serde serialization, using Foundation classes
  (`NSString`, `NSNumber`, `NSArray`, `NSDictionary`, `NSData`).
- `Serializer`, `Deserializer`, and the value types `Value`, `ArchivedObject`, `Class`,
  `Reference`, `ReferenceKind`, `StructValue`, `TypedValues`.
- `Error`: `InvalidHeader`, `UnexpectedEof`, `Syntax`, `LimitExceeded { offset, limit, actual, max }`,
  `Io`, `Custom`. Header, end-of-input, syntax, and limit errors carry the byte offset.
- `MAX_INPUT_BYTES`: 64 MiB.

## Guarantees

- Reads streamer version 4 with either byte-order signature: `streamtyped` (little-endian) or
  `typedstream` (big-endian). Any other header is `InvalidHeader`. Enforced by:
  `tests/integration.rs::rejects_invalid_header_and_truncated_values`,
  `tests/malformed.rs::rejects_bad_header_fields`.
- Real Messages blobs parse to a stable `Value`. Enforced by: 21 snapshot tests in
  `tests/fixtures.rs` over `fixtures/attributed-body-*.bin`.
- Serde data written by `to_vec` reads back equal with `from_slice`. Enforced by:
  `tests/integration.rs::round_trips_nested_serde_data`, `reader_and_writer_apis_round_trip`.
- Every decoding budget below returns `LimitExceeded` naming the budget; none allocates past it,
  and a declared array length is never used to preallocate. Enforced by: `tests/malformed.rs`
  (`enforces_the_blob_limit`, `byte_arrays_cannot_bypass_the_blob_limit`,
  `declared_array_lengths_do_not_preallocate`, `enforces_the_encoding_nesting_limit`,
  `enforces_the_encoding_item_limit`, `enforces_the_class_chain_limit`,
  `enforces_the_object_nesting_limit`).
- Readers never read more than their limit. Enforced by: `tests/integration.rs::readers_are_bounded`.
- No input panics `value_from_slice`. Enforced by: the `typedstream_parse` fuzz target.

### Not yet enforced

- The fuzz target exercises `value_from_slice` only; the Serde path (`from_slice::<T>`) and the
  writer are not fuzzed.

### Known bugs

None known.

## Limits and non-goals

| Budget | Value | Configurable |
| --- | --- | --- |
| Reader input | 64 MiB (`MAX_INPUT_BYTES`) | yes, `from_reader_with_limit` |
| Nesting depth | 128 | no |
| Single blob or string | 64 MiB | no |
| Retained objects | 2²⁰ | no |
| Shared strings | 2²⁰ | no |
| Values per stream | 2²² | no |
| Class inheritance chain | 256 | no |
| Type-encoding depth | 32 | no |
| Type-encoding items | 4096 | no |

- Only typedstream (`NSArchiver`). Keyed archives (`NSKeyedArchiver`, binary plists) are a
  different format.
- The writer always emits little-endian `streamtyped`.

## Platform and permissions

Any platform; no permissions.
