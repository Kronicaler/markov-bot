# Changelog

## 0.1.4

- Replaced a bunch of `.into_iter()` to `.iter()`
- `max_tries` default value is now `100` (from `10`)
- Fixed an off-by-one small bug
- Updated docs

## 0.1.3

- Fixed `MarkovResult` struct:
  - `.refs` is now a `Vec<usize>`
  - Removed the buggy Hash implementation
  - Added derives for Hash and (De)Serialize

## 0.1.1 - 0.1.2

- Documentation updates

## 0.1.0

- First version