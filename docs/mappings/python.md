# Python mapping

Generated from `conformance/mappings.gram` by `scripts/gen-mapping-docs.py`; do not edit.
Each row is a conformance case: the gram value, the entry class, the native type, the loss kinds
and the gram text the native value writes back as (ADR-006).

| Case | Value | Class | Native | Loss | Round trip |
|---|---|---|---|---|---|
| map-integer-zero: zero | `0` | exact | int | none | `0` |
| map-integer-max: largest 64-bit integer | `9223372036854775807` | exact | int | none | `9223372036854775807` |
| map-integer-hex: hexadecimal reads as an integer | `0x10` | exact | int | none | `16` |
| map-big-integer-u64: a big integer that fits u64 | `9223372036854775808` | exact | int | none | `9223372036854775808` |
| map-big-integer-i128: a big integer that fits i128 | `-9223372036854775809` | exact | int | none | `-9223372036854775809` |
| map-big-integer-huge: a big integer beyond 128 bits | `1000000000000000000000000000000000000000000` | exact | int | none | `1000000000000000000000000000000000000000000` |
| map-decimal: a decimal is a float | `0.1` | exact | float | none | `0.1` |
| map-decimal-whole: a whole decimal stays a decimal in Rust | `1.0` | exact | float | none | `1.0` |
| map-decimal-negative-zero: negative zero | `-0.0` | exact | float | none | `-0.0` |
| map-big-decimal: a big decimal has no std type | `0.10000000000000000001` | exact | Decimal | none | `0.10000000000000000001` |
| map-boolean: a boolean | `true` | exact | bool | none | `true` |
| map-string: a string | `"Ada Lovelace"` | exact | str | none | `"Ada Lovelace"` |
| map-tagged-unknown: an unknown tag | `ts`string`` | exact | Tagged | none | `ts`string`` |
| map-range: a range | `1.5m..2m` | exact | Range | none | `1.5m..2m` |
| map-range-open: an open range | `1...` | exact | Range | none | `1...` |
| map-measurement: a measurement | `5.0kg` | exact | Measurement | none | `5.0kg` |
| map-array: an array | `[1, 2]` | exact | list | none | `[1, 2]` |
| map-array-empty: the empty array | `[]` | exact | list | none | `[]` |
| map-map: a map | `{b: 1, a: [1, 2]}` | exact | dict | none | `{a: [1, 2], b: 1}` |
| map-date: a date | `date`2024-01-15`` | exact | Date | none | `date`2024-01-15`` |
| map-time: a time with normal content | `time`17:35:00Z`` | exact | Time | none | `time`17:35:00Z`` |
| map-datetime: a datetime | `datetime`2024-01-15T10:30:00+01:00[Europe/Paris]`` | exact | DateTime | none | `datetime`2024-01-15T10:30:00+01:00[Europe/Paris]`` |
| map-duration: a duration | `duration`PT2H3M42S`` | exact | Duration | none | `duration`PT2H3M42S`` |
| map-point: a point | `point`SRID=4326;12.5 55.7`` | exact | Point | none | `point`SRID=4326;12.5 55.7`` |
| map-uuid: a uuid | `uuid`123e4567-e89b-12d3-a456-426614174000`` | exact | Uuid | none | `uuid`123e4567-e89b-12d3-a456-426614174000`` |
| map-week: a week | `week`2024W35`` | exact | Week | none | `week`2024W35`` |
| map-integer-safe-max: largest safe JavaScript integer | `9007199254740991` | exact | int | none | `9007199254740991` |
| map-integer-beyond-2-53: one past 2^53 is a bigint, not rounded | `9007199254740993` | exact | int | none | `9007199254740993` |
| map-big-decimal-overflow: a decimal beyond the float range | `10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0` | exact | Decimal | none | `10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0` |
