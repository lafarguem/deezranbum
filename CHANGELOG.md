# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-09-03

### Fixed

- The picker no longer loses its scroll position between keypresses: the selection is no longer 
  pinned to the bottom row, moving up no longer jumps back to the top, and scrolling now starts 
  a couple of rows before the edge of the list.
- stats now lists album titles under Top Albums, counted by total plays. It previously showed 
  artist names counted by distinct albums.

## [0.2.0] - 2026-09-03

Deezer playlists can now be treated as albums, and the old `playlist` command has been renamed
to `collection` to make room for them.

### Breaking

- `deezranbum playlist …` now manages **real Deezer playlists**. The previous `playlist` command —
  local named groups of albums — is now `deezranbum collection …`. Its subcommands are otherwise
  unchanged: `collection edit`, `collection list`, `collection play`, `collection delete`.
- In the interactive picker, <kbd>space</kbd> no longer toggles selection — it types a space into
  the search box, so multi-word queries like `pink floyd` finally work. Use <kbd>tab</kbd> to
  toggle, which it already did.

### Added

- Register any public Deezer playlist into the album pool:
  - `playlist add <url|id>` — accepts a full URL or a bare id, including editorial playlists that
    aren't in your own library
  - `playlist import` — pick from your own playlists with the interactive picker
  - `playlist list`
  - `playlist remove [title]` — fuzzy match, or the picker with no argument
- A registered playlist then behaves exactly like an album: it can be picked at random, is tracked
  as seen so it won't repeat, and works with `session history`, `session remove`, `replay`, `stats`
  and membership in a collection. It is queued as a whole into the Deezer web player.
- `album --kind album|playlist` restricts a pick to one type. Omit it for both.
- Playlist metadata maps onto the existing album fields — creator as the artist, creation date as
  the release date, and summed track duration as the duration — so `--artist`, `--before`/`--after`
  and `--min-duration`/`--max-duration` all apply to playlists. Playlists carry no genre, so
  `--genre` never matches one; `--max-duration` is the way to keep very long playlists out of a pick.
- `fetch` now refreshes registered playlist metadata alongside album metadata.
- The picker shows a track count beside playlists, to tell them apart from albums at a glance.
- `deezranbum stats` — listening totals by artist and genre, plus a play-count leaderboard.

### Fixed

- In the picker, when items were already selected (editing an existing collection), deselecting
  everything and pressing <kbd>enter</kbd> silently replaced the collection with whichever row the
  cursor happened to be on. It now leaves the collection untouched. The convenience shortcut —
  <kbd>enter</kbd> confirming the highlighted row — still applies when the picker opens with
  nothing selected.
- Playlists with a zero duration are no longer re-fetched from the Deezer API on every run.

### Migration

Existing collections are migrated automatically the first time 0.2.0 runs; nothing is lost and no
action is needed. Note that once 0.2.0 has saved state, versions 0.1.8 and earlier will no longer
see your collections.

---

Releases before 0.2.0 predate this changelog; see the
[GitHub releases](https://github.com/lafarguem/deezranbum/releases) for those.

[Unreleased]: https://github.com/lafarguem/deezranbum/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/lafarguem/deezranbum/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/lafarguem/deezranbum/compare/v0.1.8...v0.2.0
