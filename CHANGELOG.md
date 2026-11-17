# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-11-18

First public release.

### Added

- IBus engine with phonetic Russian transliteration, multi-letter sequences
  (`shch`, `zh`, `kh`, `ya`, …), case-aware output and whole-word exceptions
  (`zhizn`, `lyubov`, `den`).
- Word suggestions from a bundled ~10,000-word Russian dictionary, candidate
  selection with `1`–`9` / arrow keys / `Enter`, and learning of committed words.
- Custom transliteration mappings, configurable from the settings window.
- Top-bar controller with `EN`/`RU` indicator, virtual keyboard, settings and
  about windows.
- System tray icon with mode switching, window shortcuts and quit.
- Global `Meta + R` toggle (X11).
- Engine supervisor that keeps the engine process alive and recovers after an
  `ibus-daemon` restart.
- Runtime IBus component registration, so the engine is usable immediately after
  start-up.
- "Launch on system login" support through an XDG autostart entry.
- Debian packaging with desktop entry, icons, IBus component XML and a
  `russian-keyboard-setup` helper.

[1.0.0]: https://github.com/XexanTerxcin/Russian-Keyboard-Tool/releases/tag/v1.0.0
