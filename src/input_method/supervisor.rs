use crate::input_method::activation::{get_ibus_engine, set_ibus_engine};
use ibus_engine::ibus::get_ibus_addr;
use log::{info, warn};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const ENGINE_NAME: &str = "RussianKeyboard";
pub const SERVICE_NAME: &str = "org.freedesktop.IBus.RussianKeyboard";

/// Keeps the `russian-keyboard-engine` process alive and switches the IBus engine.
///
/// The engine registers itself with the running `ibus-daemon` at startup (see
/// `register_component` in `src/bin/engine.rs`). `ibus-daemon` never rescans
/// `/usr/share/ibus/component` while it is running, so without this supervisor a
/// freshly installed engine would stay invisible until the user logged out.
pub struct EngineSupervisor {
    child: Option<Child>,
    started_at: Option<Instant>,
    rt: tokio::runtime::Handle,
}

/// How long a freshly started engine gets to register itself with `ibus-daemon`
/// before the supervisor considers it broken.
const STARTUP_GRACE: Duration = Duration::from_secs(10);

impl EngineSupervisor {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        Self {
            child: None,
            started_at: None,
            rt,
        }
    }

    /// Locate the engine binary: the installed one first, then a sibling of the GUI
    /// binary (handy when running straight out of `target/release`).
    pub fn engine_binary() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("RUSSIAN_KEYBOARD_ENGINE") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }

        let installed = PathBuf::from("/usr/bin/russian-keyboard-engine");
        if installed.is_file() {
            return Some(installed);
        }

        let exe = std::env::current_exe().ok()?;
        let sibling = exe.parent()?.join("russian-keyboard-engine");
        sibling.is_file().then_some(sibling)
    }

    /// Is the engine process we started still alive?
    pub fn is_running(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };

        match child.try_wait() {
            Ok(None) => true,
            _ => {
                self.child = None;
                self.started_at = None;
                false
            }
        }
    }

    /// True when some process already owns `org.freedesktop.IBus.RussianKeyboard`
    /// on the IBus bus (for example the engine of a previous GUI instance, or one
    /// started by `ibus-daemon` itself).
    fn service_is_owned(&self) -> bool {
        self.rt.block_on(async {
            let Ok(addr) = get_ibus_addr() else {
                return false;
            };
            let Ok(builder) = zbus::connection::Builder::address(addr.as_str()) else {
                return false;
            };
            let Ok(conn) = builder.build().await else {
                return false;
            };
            let Ok(reply) = conn
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "NameHasOwner",
                    &(SERVICE_NAME,),
                )
                .await
            else {
                return false;
            };

            reply.body().deserialize::<bool>().unwrap_or(false)
        })
    }

    /// Remove leftover engine processes from an earlier GUI run.
    ///
    /// The kernel truncates `comm` to 15 bytes and both binaries start with the
    /// same prefix, so the full command line is matched instead. `pgrep` never
    /// matches its own process. An engine that already owns the bus name is in
    /// use and is left alone.
    pub fn kill_stale_engines(&mut self) {
        if self.child.is_some() || self.service_is_owned() {
            return;
        }

        let Ok(out) = Command::new("pgrep")
            .args(["-f", "russian-keyboard-engine"])
            .output()
        else {
            return;
        };

        let pids = String::from_utf8_lossy(&out.stdout).to_string();

        for pid in pids.split_whitespace() {
            info!("stopping leftover engine process {pid}");
            let _ = Command::new("kill").arg(pid).status();
        }

        std::thread::sleep(Duration::from_millis(250));
    }

    /// Start the engine when it is not running yet. Returns `true` when an engine
    /// is registered on the IBus bus.
    ///
    /// This also recovers from an `ibus-daemon` restart: a live child that no
    /// longer owns the bus name is attached to a dead connection and is replaced.
    pub fn ensure_running(&mut self) -> bool {
        let owned = self.service_is_owned();
        let alive = self.is_running();

        if alive {
            if owned {
                return true;
            }
            // The engine may simply still be connecting; do not kill it yet.
            if self
                .started_at
                .is_some_and(|started| started.elapsed() < STARTUP_GRACE)
            {
                return true;
            }
            if let Some(mut child) = self.child.take() {
                info!("engine lost its IBus registration; restarting it");
                let _ = child.kill();
                let _ = child.wait();
                self.started_at = None;
            }
        } else if owned {
            // Started by an earlier GUI instance or by ibus-daemon itself.
            return true;
        }

        let Some(binary) = Self::engine_binary() else {
            warn!("could not find the russian-keyboard-engine binary");
            return false;
        };

        match Command::new(&binary)
            .arg("--ibus")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                info!("started {} (pid {})", binary.display(), child.id());
                self.child = Some(child);
                self.started_at = Some(Instant::now());
                true
            }
            Err(e) => {
                warn!("failed to start {}: {e}", binary.display());
                false
            }
        }
    }

    /// Switch the global IBus engine, starting the engine if needed.
    pub fn apply(&mut self, russian: bool) -> Result<(), String> {
        if russian {
            // The engine may need a moment to register itself after start-up, so retry.
            for attempt in 0..5 {
                if set_ibus_engine(ENGINE_NAME) {
                    return Ok(());
                }

                if !self.ensure_running() && attempt == 0 {
                    warn!("engine process is not running; will keep retrying");
                }

                std::thread::sleep(Duration::from_millis(150 * (attempt + 1)));
            }

            Err("Could not activate Russian input. Is IBus running?".to_string())
        } else {
            if get_ibus_engine().is_some_and(|current| current != ENGINE_NAME) {
                return Ok(());
            }

            for name in ["xkb:us::eng", "xkb:us:intl:eng", "xkb:us:dvorak:eng"] {
                if set_ibus_engine(name) {
                    return Ok(());
                }
            }

            Err("Could not switch back to the English layout".to_string())
        }
    }

    /// Keep the engine process healthy and, when the process had to be restarted
    /// while Russian mode is active, make sure IBus points at it again.
    pub fn revive_if_needed(&mut self, russian: bool) {
        let was_running = self.is_running();
        if !self.ensure_running() {
            return;
        }

        if russian
            && !was_running
            && self.child.is_some()
            && get_ibus_engine().as_deref() != Some(ENGINE_NAME)
        {
            let _ = self.apply(true);
        }
    }
}

impl Drop for EngineSupervisor {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
