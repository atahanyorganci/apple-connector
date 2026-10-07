# `serde-vcard`: RFC 6350 vCard reader and writer

Reads vCard 3.0 and 4.0 text into `VCard` values and writes vCard 4.0. `apple-connector` uses it
for the `/vcard` contact routes, and `serde-carddav` for embedded `address-data`.

## Kind

Codec.

## Public API

- `from_str(&str)`, `from_slice(&[u8])`, `parse_vcard(&[u8])` → `Result<VCard>`: the first card.
- `parse_vcards(&str) -> Result<Vec<VCard>>`: every card.
- `from_reader(R)`, `from_reader_with_limit(R, limit)`.
- `to_string(&VCard)`, `to_writer(W, &VCard)`.
- Model: `VCard`, `StructuredName`, `Telephone`, `Email`, `Address`, `Url`, `SocialProfile`,
  `DateOrDateTime`, `Photo`, `ExtensionBag`, `RawProperty`.
- `Error` (via `thiserror`), including `LimitExceeded { limit, actual, max }`. A line with no value
  separator and an invalid `PHOTO` are errors that name the source line.
- `MAX_INPUT_BYTES`: 16 MiB.

## Guarantees

- Reads 3.0 and 4.0; always writes `VERSION:4.0` ([VCARD-0001](decisions/VCARD-0001-version-policy.md)).
  Enforced by: `tests/spec.rs::output_is_always_version_4`.
- A full card round-trips without loss. Enforced by:
  `tests/properties.rs::a_full_card_round_trips_without_loss`.
- `URL` and `X-SOCIALPROFILE` parse and serialize; `IMPP` is read as a social profile. Enforced by:
  `tests/properties.rs::urls_parse_and_serialize`, `social_profiles_parse_and_serialize`,
  `impp_is_read_as_a_social_profile`.
- Properties without a model field are preserved with their group and parameters and written
  back. Enforced by: `tests/properties.rs::unknown_properties_are_preserved`.
- A 3.0 `ENCODING=b` photo becomes a vCard 4 `data:` URI; a 4.0 `data:` URI photo parses; URI
  photos stay URIs. Enforced by: `tests/properties.rs::v3_photo_input_becomes_v4_output`,
  `v4_data_uri_photo_parses_instead_of_failing_the_whole_card`, `photo_uris_stay_uris`.
- Apple `itemN.X-ABLabel` groups become the property's label; bare `TYPE` tokens are understood;
  `TYPE` is written once per property; quoted parameter values may contain `:`. Enforced by:
  `tests/properties.rs` (`apple_group_labels_become_the_property_label`,
  `bare_type_parameters_are_understood`, `type_is_written_once_per_property`,
  `quoted_parameter_values_may_contain_a_colon`).
- Folded lines unfold without eating significant spaces; long lines are folded so no line exceeds
  75 octets, without splitting a character. Enforced by:
  `tests/properties.rs::folded_values_keep_significant_spaces`,
  `tests/spec.rs::long_lines_fold_at_75_octets_without_splitting_characters`.
- An unterminated card, a nested `BEGIN:VCARD`, and `END:VCARD` without `BEGIN` are errors.
  Enforced by: `tests/properties.rs::unterminated_cards_are_an_error`,
  `tests/spec.rs::unbalanced_card_markers_are_errors`.
- `from_str` returns the first card; `parse_vcards` returns all. Enforced by:
  `tests/spec.rs::from_str_returns_only_the_first_card`, `tests/properties.rs::multiple_cards_still_parse`.
- Readers never read more than their limit. Enforced by: `tests/limits.rs`.
- No input panics `parse_vcards` followed by `to_string`. Enforced by: the `vcard_parse` fuzz
  target.

### Known bugs

- Output lines end in LF; RFC 6350 §3.2 requires CRLF. Proven by:
  `tests/spec.rs::lines_end_with_crlf` (ignored, fails today). Tracked in #164.
- A `BDAY` the date parser cannot read (for example the year-less `--0415` RFC 6350 allows) is
  dropped: it is neither a birthday nor kept in `unknown`. Proven by:
  `tests/spec.rs::an_unparseable_birthday_is_not_dropped` (ignored, fails today). Tracked in #165.
- The `Cargo.toml` description still says "Serde Serializer and Deserializer"; the crate has none
  since #94 ([REC-0013](../../../docs/decisions/REC-0013-typed-format-apis.md)). Tracked in #171.

## Limits and non-goals

- `IMPP` is written back as `X-SOCIALPROFILE`
  (`tests/spec.rs::impp_is_written_back_as_x_socialprofile`).
- An invalid `PHOTO` fails the whole card.
- `BDAY` accepts `YYYYMMDD`, `YYYY-MM-DD`, and RFC 3339 date-times only.
- vCard 2.1 is not a goal; its bare type parameters happen to parse.
- Lines outside `BEGIN:VCARD` … `END:VCARD` are ignored.

## Platform and permissions

Any platform; no permissions.
