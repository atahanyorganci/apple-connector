---
id: VCARD-0001
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/58
  - https://github.com/atahanyorganci/apple-connector/issues/93
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/65
  - https://github.com/atahanyorganci/apple-connector/pull/140
supersedes: []
superseded-by: []
---

# The vCard reader accepts 3.0 and 4.0 input and the writer always emits 4.0, keeping unknown properties.

## Question

Apple exports vCard 3.0; RFC 6350 is 4.0. Clients send either. What does the crate read and write?

## Options

- **Round-trip the input version**: two writers to maintain.
- **Read both, write 4.0.**

## Decision

Read both, write 4.0. Version-specific syntax is normalised on read: bare `TYPE` tokens
(`TEL;WORK:`), Apple's `itemN.X-ABLabel` groups, and vCard 3.0 `ENCODING=b` photos. The writer
emits `VERSION:4.0` and vCard 4 forms, including `PHOTO:data:<media>;base64,…`. Properties without
a model field are preserved in `unknown` (with group and parameters) and written back; `X-`
properties go to `extensions`.

## Consequences

- A 3.0 card comes back as 4.0.
- `IMPP` is read into `social_profiles` and written as `X-SOCIALPROFILE`, so the property name does
  not survive.
- A value the model cannot parse can still be lost: an unparseable `BDAY` is dropped (known bug).

## Evidence

- `packages/serde-vcard/tests/properties.rs`: `v3_photo_input_becomes_v4_output`,
  `unknown_properties_are_preserved`, `bare_type_parameters_are_understood`,
  `apple_group_labels_become_the_property_label`.
- `packages/serde-vcard/tests/spec.rs`: `output_is_always_version_4`,
  `impp_is_written_back_as_x_socialprofile`, `an_unparseable_birthday_is_not_dropped` (ignored,
  fails today).
