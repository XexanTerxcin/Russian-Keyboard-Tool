use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

pub struct ShortcutListener {
    running: Arc<AtomicBool>,
}

impl ShortcutListener {
    pub fn start<F>(on_triggered: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        thread::spawn(move || {
            let (conn, screen_num) = match x11rb::connect(None) {
                Ok(c) => c,
                Err(e) => {
                    error!("X11 shortcut listener: could not connect to X11: {}", e);
                    return;
                }
            };

            let screen = &conn.setup().roots[screen_num];
            let root = screen.root;

            // Keycode for 'r' on standard QWERTY keyboards: 27
            let keycode = 27u8;

            // ModMask: Mod4 (Super/Meta) with combinations of CapsLock (Lock) and NumLock (Mod2)
            let mod_combinations = [
                ModMask::M4,
                ModMask::M4 | ModMask::LOCK,
                ModMask::M4 | ModMask::M2,
                ModMask::M4 | ModMask::M2 | ModMask::LOCK,
            ];

            for &m in &mod_combinations {
                let _ = conn.grab_key(false, root, m, keycode, GrabMode::ASYNC, GrabMode::ASYNC);
            }
            let _ = conn.flush();
            info!("X11 shortcut listener active for Meta+R");

            while running_clone.load(Ordering::Relaxed) {
                match conn.poll_for_event() {
                    Ok(Some(x11rb::protocol::Event::KeyPress(ev))) => {
                        if ev.detail == keycode {
                            info!("Meta+R detected globally via X11");
                            on_triggered();
                        }
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(e) => {
                        error!("X11 event error: {}", e);
                        break;
                    }
                }
            }

            for &m in &mod_combinations {
                let _ = conn.ungrab_key(keycode, root, m);
            }
            let _ = conn.flush();
        });

        Self { running }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
