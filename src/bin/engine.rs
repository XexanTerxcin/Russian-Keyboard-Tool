use std::collections::HashMap;

use ibus_engine::ibus::{
    get_ibus_addr, IBus, IBusEngine, IBusEngineBackend, IBusFactory, IBusModifierState,
    IBusPreeditFocusMode, LookupTable,
};
use log::{info, warn};
use russian_keyboard_tool::config::AppSettings;
use russian_keyboard_tool::transliteration::Engine as TransEngine;
use std::future::Future;
use zbus::object_server::{ObjectServer, SignalEmitter};
use zbus::zvariant::{Structure, StructureBuilder, Value};

const ENGINE_NAME: &str = "RussianKeyboard";
const SERVICE_NAME: &str = "org.freedesktop.IBus.RussianKeyboard";
const COMPONENT_NAME: &str = "org.freedesktop.IBus.RussianKeyboard";
const IBUS_SERVICE: &str = "org.freedesktop.IBus";
const IBUS_PATH: &str = "/org/freedesktop/IBus";
const ICON_PATH: &str = "/usr/share/pixmaps/russian-keyboard-tool.png";

fn icon_path() -> String {
    if std::path::Path::new(ICON_PATH).exists() {
        ICON_PATH.to_string()
    } else {
        String::new()
    }
}

fn engine_exec() -> String {
    match std::env::current_exe() {
        Ok(p) => format!("{} --ibus", p.display()),
        Err(_) => "/usr/bin/russian-keyboard-engine --ibus".to_string(),
    }
}

/// Build the `IBusEngineDesc` structure exactly as `ibusenginedesc.c` serializes it.
fn build_engine_desc() -> Structure<'static> {
    StructureBuilder::new()
        .add_field("IBusEngineDesc")
        .add_field(HashMap::<String, Value<'static>>::new())
        // name, longname, description, language, license, author, icon, layout
        .add_field(ENGINE_NAME.to_string())
        .add_field("Russian Keyboard".to_string())
        .add_field("Russian phonetic transliteration input method".to_string())
        .add_field("ru".to_string())
        .add_field("MIT".to_string())
        .add_field("Xexan Terxcin".to_string())
        .add_field(icon_path())
        .add_field("us".to_string())
        .add_field(99u32)
        // hotkeys, symbol, setup, layout_variant, layout_option, version, textdomain, icon_prop_key
        .add_field(String::new())
        .add_field(String::new())
        .add_field(String::new())
        .add_field(String::new())
        .add_field(String::new())
        .add_field("1.0.0".to_string())
        .add_field("russian-keyboard-tool".to_string())
        .add_field(String::new())
        .build()
        .expect("valid engine desc structure")
}

/// Build the `IBusComponent` structure exactly as `ibuscomponent.c` serializes it.
fn build_component() -> Structure<'static> {
    StructureBuilder::new()
        .add_field("IBusComponent")
        .add_field(HashMap::<String, Value<'static>>::new())
        // name, description, version, license, author, homepage, exec, textdomain
        .add_field(COMPONENT_NAME)
        .add_field("Russian Keyboard Tool".to_string())
        .add_field("1.0.0".to_string())
        .add_field("MIT".to_string())
        .add_field("Xexan Terxcin".to_string())
        .add_field("https://github.com/XexanTerxcin/Russian-Keyboard-Tool".to_string())
        .add_field(engine_exec())
        .add_field("russian-keyboard-tool".to_string())
        // observed paths (av) and engine descs (av)
        .add_field(Vec::<Value<'static>>::new())
        .add_field(vec![Value::new(build_engine_desc())])
        .build()
        .expect("valid component structure")
}

/// Register this engine with the running `ibus-daemon` at runtime.
///
/// `ibus-daemon` only scans `/usr/share/ibus/component` when it starts, so a freshly
/// installed engine would otherwise be invisible until the whole IBus daemon is
/// restarted (or the user logs out). Calling `RegisterComponent` from the engine's own
/// D-Bus connection makes the engine usable immediately: the daemon builds its factory
/// proxy against *our* connection, which is exactly where `IBus/Factory` lives.
async fn register_component(conn: &zbus::Connection) {
    let component = Value::new(build_component());
    let body = (component,);

    match conn
        .call_method(
            Some(IBUS_SERVICE),
            IBUS_PATH,
            Some(IBUS_SERVICE),
            "RegisterComponent",
            &body,
        )
        .await
    {
        Ok(_) => info!("registered component {COMPONENT_NAME} with ibus-daemon at runtime"),
        Err(e) => warn!("runtime RegisterComponent failed: {e}"),
    }
}

#[derive(Clone)]
struct RussianEngine {
    buffer: String,
    engine: TransEngine,
    lookup_table: LookupTable,
}

impl RussianEngine {
    fn new(settings: &AppSettings) -> Self {
        let table =
            LookupTable::new(Vec::new(), 5, true, true).expect("Could not create LookupTable");
        let mut engine = TransEngine::new();
        engine.set_suggestions_enabled(settings.enable_suggestions);
        engine.set_auto_capitalize(settings.auto_capitalization);
        Self {
            buffer: String::new(),
            engine,
            lookup_table: table,
        }
    }

    fn current_word(&self) -> String {
        self.engine
            .get_suggestions(&self.buffer)
            .into_iter()
            .next()
            .unwrap_or_else(|| self.engine.transliterate(&self.buffer))
    }

    async fn show_state(
        se: &SignalEmitter<'_>,
        engine: &TransEngine,
        buffer: &str,
        table: &LookupTable,
    ) {
        if buffer.is_empty() {
            let _ = <RussianEngine as IBusEngineBackend>::update_preedit_text(
                se,
                String::new(),
                0,
                false,
                IBusPreeditFocusMode::Clear,
            )
            .await;
            let _ =
                <RussianEngine as IBusEngineBackend>::update_lookup_table(se, table, false).await;
            return;
        }

        let pre = engine.transliterate(buffer);
        let _ = <RussianEngine as IBusEngineBackend>::update_preedit_text(
            se,
            pre.clone(),
            pre.chars().count() as u32,
            true,
            IBusPreeditFocusMode::Clear,
        )
        .await;
        let _ = <RussianEngine as IBusEngineBackend>::update_lookup_table(
            se,
            table,
            !table.candidates().is_empty(),
        )
        .await;
    }

    async fn flush(
        se: &SignalEmitter<'_>,
        engine: &TransEngine,
        buffer: &mut String,
        table: &mut LookupTable,
        learn: bool,
    ) {
        if buffer.is_empty() {
            return;
        }
        let out = engine.transliterate(buffer);
        if learn {
            // Learning happens through the shared engine in the caller; kept for clarity.
        }
        let _ = <RussianEngine as IBusEngineBackend>::commit_text(se, out).await;
        buffer.clear();
        table.clear();
        let _ = <RussianEngine as IBusEngineBackend>::update_preedit_text(
            se,
            String::new(),
            0,
            false,
            IBusPreeditFocusMode::Clear,
        )
        .await;
        let _ = <RussianEngine as IBusEngineBackend>::update_lookup_table(se, table, false).await;
    }
}

struct Factory {
    settings: AppSettings,
}

impl IBusFactory<RussianEngine> for Factory {
    fn create_engine(&mut self, name: String) -> Result<RussianEngine, String> {
        if name == ENGINE_NAME || name.is_empty() {
            info!("creating a RussianKeyboard engine instance");
            Ok(RussianEngine::new(&self.settings))
        } else {
            warn!("Unknown engine requested: {}", name);
            Err(format!("unknown engine: {}", name))
        }
    }
}

const KEY_BACKSPACE: u32 = 0xff08;
const KEY_RETURN: u32 = 0xff0d;
const KEY_TAB: u32 = 0xff09;
const KEY_SPACE: u32 = 0x0020;
const KEY_UP: u32 = 0xff52;
const KEY_DOWN: u32 = 0xff54;
const KEY_ESCAPE: u32 = 0xff1b;

impl IBusEngine for RussianEngine {
    fn process_key_event(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
        keyval: ibus_engine::ibus::xkeysym::Keysym,
        _keycode: ibus_engine::ibus::xkeysym::KeyCode,
        state: IBusModifierState,
    ) -> impl Future<Output = zbus::fdo::Result<bool>> + Send {
        let ks = keyval.raw();
        // `state` carries a release bit; we only act on key presses.
        let is_release = state.release();
        let has_special = state.control() || state.mod1() || state.mod4() || state.super_();

        async move {
            if is_release {
                return Ok(false);
            }

            // Never swallow shortcuts that involve Ctrl / Alt / Super.
            if has_special {
                RussianEngine::flush(
                    &se,
                    &self.engine,
                    &mut self.buffer,
                    &mut self.lookup_table,
                    true,
                )
                .await;
                return Ok(false);
            }

            // Escape: discard the current composition.
            if ks == KEY_ESCAPE && !self.buffer.is_empty() {
                self.buffer.clear();
                self.lookup_table.clear();
                RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table)
                    .await;
                return Ok(true);
            }

            if ks == KEY_BACKSPACE {
                if !self.buffer.is_empty() {
                    self.buffer.pop();
                    self.lookup_table
                        .reset_candidates(self.engine.get_suggestions(&self.buffer));
                    RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table)
                        .await;
                    return Ok(true);
                }
                return Ok(false);
            }

            // Candidate navigation while a composition is active.
            if !self.buffer.is_empty() && !self.lookup_table.candidates().is_empty() {
                if ks == KEY_DOWN {
                    self.lookup_table.cursor_down();
                    let _ = <RussianEngine as IBusEngineBackend>::update_lookup_table(
                        &se,
                        &self.lookup_table,
                        true,
                    )
                    .await;
                    return Ok(true);
                }
                if ks == KEY_UP {
                    self.lookup_table.cursor_up();
                    let _ = <RussianEngine as IBusEngineBackend>::update_lookup_table(
                        &se,
                        &self.lookup_table,
                        true,
                    )
                    .await;
                    return Ok(true);
                }
                // 1-9 pick a candidate from the current page.
                if (0x0031..=0x0039).contains(&ks) {
                    let idx = ks - 0x0031;
                    if let Some(cand) = self.lookup_table.get_candidate_by_index_in_page(idx) {
                        let picked = cand.clone();
                        self.engine.learn_word(&picked);
                        let _ =
                            <RussianEngine as IBusEngineBackend>::commit_text(&se, picked).await;
                        self.buffer.clear();
                        self.lookup_table.clear();
                        RussianEngine::show_state(
                            &se,
                            &self.engine,
                            &self.buffer,
                            &self.lookup_table,
                        )
                        .await;
                        return Ok(true);
                    }
                }
            }

            if ks == KEY_RETURN || ks == KEY_TAB || ks == KEY_SPACE {
                if !self.buffer.is_empty() {
                    let selected = if self.lookup_table.candidates().is_empty() {
                        None
                    } else if ks == KEY_RETURN {
                        self.lookup_table
                            .get_candidate_by_index_in_page(self.lookup_table.cursor_pos_in_page())
                            .cloned()
                    } else {
                        None
                    };

                    let text = match selected {
                        Some(text) => text,
                        None => self.current_word(),
                    };

                    self.engine.learn_word(&text);
                    let _ = <RussianEngine as IBusEngineBackend>::commit_text(&se, text).await;
                    self.buffer.clear();
                    self.lookup_table.clear();
                    RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table)
                        .await;

                    // Space must still produce a space in the target application.
                    return Ok(ks != KEY_SPACE);
                }
                return Ok(false);
            }

            // Printable ASCII letters and the soft/hard sign quotes.
            if let Some(ch) = keyval.key_char() {
                if ch.is_ascii_alphabetic() || ch == '\'' || ch == '"' {
                    self.buffer.push(ch);
                    self.lookup_table
                        .reset_candidates(self.engine.get_suggestions(&self.buffer));
                    RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table)
                        .await;
                    return Ok(true);
                }

                // Any other printable character ends the composition and is typed normally.
                if !self.buffer.is_empty() {
                    let out = self.current_word();
                    self.engine.learn_word(&out);
                    let _ = <RussianEngine as IBusEngineBackend>::commit_text(&se, out).await;
                    self.buffer.clear();
                    self.lookup_table.clear();
                    RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table)
                        .await;
                }
                return Ok(false);
            }

            Ok(false)
        }
    }

    fn reset(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = zbus::fdo::Result<()>> + Send {
        async move {
            self.buffer.clear();
            self.lookup_table.clear();
            RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table).await;
            Ok(())
        }
    }

    fn focus_in(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = zbus::fdo::Result<()>> + Send {
        async move {
            self.buffer.clear();
            self.lookup_table.clear();
            RussianEngine::show_state(&se, &self.engine, &self.buffer, &self.lookup_table).await;
            Ok(())
        }
    }

    fn focus_out(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = zbus::fdo::Result<()>> + Send {
        async move {
            RussianEngine::flush(
                &se,
                &self.engine,
                &mut self.buffer,
                &mut self.lookup_table,
                false,
            )
            .await;
            Ok(())
        }
    }

    fn enable(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = zbus::fdo::Result<()>> + Send {
        async move {
            self.buffer.clear();
            self.lookup_table.clear();
            Ok(())
        }
    }

    fn disable(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = zbus::fdo::Result<()>> + Send {
        async move {
            RussianEngine::flush(
                &se,
                &self.engine,
                &mut self.buffer,
                &mut self.lookup_table,
                false,
            )
            .await;
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = match get_ibus_addr() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("russian-keyboard-engine: cannot find IBus address: {e}");
            return;
        }
    };

    let settings = AppSettings::load();
    let bus = match IBus::new(addr, Factory { settings }, SERVICE_NAME.to_string()).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("russian-keyboard-engine: cannot connect to ibus-daemon: {e}");
            return;
        }
    };

    register_component(&bus.conn()).await;
    info!("Russian Keyboard engine running");

    std::future::pending::<()>().await;
}
