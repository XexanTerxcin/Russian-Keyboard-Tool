# Architecture

Russian Keyboard Tool is two binaries plus a small shared library:

| Binary | Role |
| --- | --- |
| `russian-keyboard-tool` | GUI controller: top bar, windows, tray, `Meta + R` listener, engine supervisor |
| `russian-keyboard-engine` | IBus engine: receives key events and commits Cyrillic text |

The shared library (`src/transliteration/`) contains the parser, the dictionary and the
suggestion engine. It has no GUI or D-Bus dependencies and is unit-tested separately.

## Process model

```
GUI process ──spawns/supervises──▶ engine process
     │                                  │
     │ `ibus engine <name>`             │ D-Bus: Engine + Factory interfaces
     ▼                                  ▼
               ibus-daemon  ◀────────────┘
                    │
                    ▼
            focused application
```

- The **GUI** is optional for typing: once the engine is registered and selected, it
  keeps working if the GUI is closed (IBus spawns the engine from the component's
  `exec` line when needed).
- The **engine** owns the D-Bus name `org.freedesktop.IBus.RussianKeyboard` and exports
  `/org/freedesktop/IBus/Factory` on its own connection.
- **ibus-daemon** routes `ProcessKeyEvent` to the active engine and applies the engine's
  `CommitText`/`UpdatePreeditText` calls to the focused application.

## Engine registration

`ibus-daemon` scans `/usr/share/ibus/component/*.xml` only when it starts. A freshly
installed engine would therefore be invisible until the next login — and
`SetGlobalEngine` would fail.

To avoid that, the engine calls `RegisterComponent` on the running daemon at start-up,
from **its own connection**:

```rust
conn.call_method(
    Some("org.freedesktop.IBus"),
    "/org/freedesktop/IBus",
    Some("org.freedesktop.IBus"),
    "RegisterComponent",
    &(component,),
).await
```

The daemon keeps a factory proxy pointing at the caller's connection, which is exactly
where the engine's `/org/freedesktop/IBus/Factory` object lives. Calling this from a
third-party connection fails with *"Object does not exist at path
/org/freedesktop/IBus/Factory"*.

The `IBusComponent` and `IBusEngineDesc` values are serialized by hand with zbus'
`StructureBuilder` (a tuple cannot hold the 16 fields of `IBusEngineDesc`), mirroring
`ibuscomponent.c` and `ibusenginedesc.c`.

Note that runtime registration adds the engine to `register_engine_list`; the list in
**IBus Preferences** comes from the registry (`engine_table`) and therefore still
requires one `ibus restart` (or a re-login) after installation. The `.deb` ships
`/usr/share/ibus/component/russian-keyboard.xml` for exactly that.

## Key handling

`RussianEngine` keeps a small state machine:

- printable ASCII letters, `'` and `"` are appended to `buffer` and re-rendered as a
  preedit string;
- the lookup table is refreshed with suggestions for the buffer;
- `1`–`9` commit a candidate, `↑`/`↓` move the candidate cursor;
- `Backspace` removes the last buffered character;
- `Esc` clears the composition;
- `Space`, `Enter`, `Tab` and punctuation commit the transliteration (space is passed
  through so the application still receives it);
- `Ctrl`/`Alt`/`Super` combinations are never swallowed.

## Transliteration

`TransliterationParser` holds rules sorted by pattern length, so the longest match wins
(`shch` before `sh` before `s`). Non-letter characters are passed through, which lets
punctuation commit the current word naturally. A handful of whole words with a soft
sign (`zhizn`, `lyubov`, `den`) are special-cased.

Case handling (`apply_case`) reproduces the input casing: `PRIVET` → `ПРИВЕТ`,
`Privet` → `Привет`, `privet` → `привет`. It can be disabled from the settings.

`SuggestionEngine` looks up the transliterated prefix in a sorted word list
(`resources/dictionaries/russian_words.txt`, embedded at compile time with
`include_str!`), returns up to seven candidates, and learns words the user commits.
User words are stored in memory for the lifetime of the engine process.

## GUI

- **Top bar** — an always-on-top, 560×78 borderless window with the mode indicator and
  buttons. Frames are rendered with `eframe`/`egui` (glow, X11).
- **Child windows** — Settings, Virtual Keyboard and About are real OS windows created
  with `show_viewport_immediate`, not nested `egui::Window`s.
- **Shortcut** — a dedicated thread opens its own X11 connection, grabs `Meta + R`
  (including CapsLock/NumLock variants) and sets an `AtomicBool` that the UI thread
  polls, so all engine switching happens on one thread.
- **Tray** — `ksni` publishes a StatusNotifierItem with an embedded ARGB pixmap.
- **Supervisor** — every three seconds it checks that the engine process is alive and
  still owns its D-Bus name, restarts it when it does not (for example after an
  `ibus-daemon` restart), and mirrors engine changes made from the IBus panel so the
  GUI never fights a manual switch.

## Configuration

`AppSettings` is a serde struct stored at
`$XDG_CONFIG_HOME/russian-keyboard-tool/settings.json`
(`~/.config/russian-keyboard-tool/settings.json`). Saving also creates or removes
`~/.config/autostart/russian-keyboard-tool.desktop` when *Launch on system login*
changes. The engine reads the same file when it is created, so suggestion and
capitalisation settings apply the next time Russian mode is activated.

## Packaging

`scripts/build-deb.sh` builds both binaries, copies `packaging/deb/` into
`dist/deb-root/`, installs the binaries into it, substitutes the target architecture
into `DEBIAN/control`, and calls `dpkg-deb --build`. The package installs:

| Path | Contents |
| --- | --- |
| `/usr/bin/russian-keyboard-tool` | GUI |
| `/usr/bin/russian-keyboard-engine` | IBus engine |
| `/usr/bin/russian-keyboard-setup` | per-user setup helper |
| `/usr/share/ibus/component/russian-keyboard.xml` | IBus component description |
| `/usr/share/applications/russian-keyboard-tool.desktop` | desktop entry |
| `/usr/share/icons/hicolor/256x256/apps/russian-keyboard-tool.png` | application icon |
| `/usr/share/pixmaps/russian-keyboard-tool.png` | engine icon |

`postinst` refreshes the system IBus cache and icon/desktop caches; `postrm` does the
same after removal.

## Tests

`tests/transliteration_tests.rs` covers the transliteration examples from the project
requirements, mixed case and punctuation, custom mappings and suggestion lookup. Run
them with `cargo test`.
