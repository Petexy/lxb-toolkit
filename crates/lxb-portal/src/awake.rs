use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use crate::dbus::{Bus, Message, Value};

const PATIENCE: Duration = Duration::from_secs(5);

type Place = (&'static str, &'static str, &'static str);

const SCREEN: Place = (
    "org.freedesktop.ScreenSaver",
    "/org/freedesktop/ScreenSaver",
    "org.freedesktop.ScreenSaver",
);
const SLEEP: Place = (
    "org.freedesktop.PowerManagement",
    "/org/freedesktop/PowerManagement/Inhibit",
    "org.freedesktop.PowerManagement.Inhibit",
);
const DESKTOP: &str = "org.freedesktop.portal.Desktop";
const DESKTOP_PATH: &str = "/org/freedesktop/portal/desktop";
const INHIBIT: &str = "org.freedesktop.portal.Inhibit";
const REQUEST: &str = "org.freedesktop.portal.Request";
const PORTAL_SUSPEND: u32 = 4;
const PORTAL_IDLE: u32 = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hold {
    pub screen: bool,
    pub sleep: bool,
}

impl Hold {
    pub const NOTHING: Hold = Hold {
        screen: false,
        sleep: false,
    };
    pub const SCREEN: Hold = Hold {
        screen: true,
        sleep: false,
    };
    pub const SLEEP: Hold = Hold {
        screen: false,
        sleep: true,
    };
    pub const SCREEN_AND_SLEEP: Hold = Hold {
        screen: true,
        sleep: true,
    };

    pub fn any(self) -> bool {
        self.screen || self.sleep
    }
}

#[derive(Debug, Default)]
pub struct Awake {
    application: String,
    reason: String,
    held: Hold,
    worker: Option<Sender<Hold>>,
}

impl Awake {
    pub fn new(application: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            application: application.into(),
            reason: reason.into(),
            held: Hold::NOTHING,
            worker: None,
        }
    }

    pub fn named(&mut self, application: impl Into<String>, reason: impl Into<String>) {
        self.application = application.into();
        self.reason = reason.into();
    }

    pub fn hold(&mut self, wanted: Hold) {
        if wanted == self.held {
            return;
        }
        self.held = wanted;
        if self.worker.is_none() {
            if !wanted.any() {
                return;
            }
            self.worker = start(self.application.clone(), self.reason.clone());
        }
        if let Some(worker) = &self.worker {
            let _ = worker.send(wanted);
        }
    }

    pub fn held(&self) -> Hold {
        self.held
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Taken {
    Cookie(Place, u32),
    Request(String),
}

fn start(application: String, reason: String) -> Option<Sender<Hold>> {
    let (tx, rx) = mpsc::channel::<Hold>();
    let application = if application.is_empty() {
        std::env::args()
            .next()
            .and_then(|path| {
                Path::new(&path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "application".to_string())
    } else {
        application
    };
    let reason = if reason.is_empty() {
        application.clone()
    } else {
        reason
    };
    std::thread::Builder::new()
        .name("lxb-awake".into())
        .spawn(move || {
            let mut bus = match Bus::session() {
                Ok(bus) => bus,
                Err(err) => {
                    eprintln!("the screen cannot be kept on: {err}");
                    return;
                }
            };
            let sandboxed = Path::new("/.flatpak-info").exists();
            let mut screen = None;
            let mut sleep = None;
            for wanted in rx {
                let asked = (application.as_str(), reason.as_str(), sandboxed);
                keep(
                    &mut bus,
                    &mut screen,
                    wanted.screen,
                    SCREEN,
                    PORTAL_IDLE,
                    asked,
                );
                keep(
                    &mut bus,
                    &mut sleep,
                    wanted.sleep,
                    SLEEP,
                    PORTAL_SUSPEND,
                    asked,
                );
            }
            for taken in [screen.take(), sleep.take()].into_iter().flatten() {
                release(&mut bus, taken);
            }
        })
        .ok()
        .map(|_| tx)
}

fn keep(
    bus: &mut Bus,
    slot: &mut Option<Taken>,
    wanted: bool,
    place: Place,
    flag: u32,
    (application, reason, sandboxed): (&str, &str, bool),
) {
    match (wanted, slot.take()) {
        (true, Some(taken)) => *slot = Some(taken),
        (true, None) => {
            *slot = (!sandboxed)
                .then(|| take(bus, place, application, reason))
                .flatten()
                .or_else(|| ask_the_portal(bus, flag, reason));
        }
        (false, Some(taken)) => release(bus, taken),
        (false, None) => {}
    }
}

fn take(bus: &mut Bus, place: Place, application: &str, reason: &str) -> Option<Taken> {
    let (name, path, interface) = place;
    let answer = bus
        .call(
            Message::call(name, path, interface, "Inhibit")
                .with(vec![Value::text(application), Value::text(reason)]),
            PATIENCE,
        )
        .ok()?;
    let cookie = answer.body.first().and_then(Value::as_u32)?;
    Some(Taken::Cookie(place, cookie))
}

fn ask_the_portal(bus: &mut Bus, flag: u32, reason: &str) -> Option<Taken> {
    let answer = bus
        .call(
            Message::call(DESKTOP, DESKTOP_PATH, INHIBIT, "Inhibit").with(vec![
                Value::text(""),
                Value::U32(flag),
                Value::options(vec![("reason", Value::text(reason))]),
            ]),
            PATIENCE,
        )
        .ok()?;
    let handle = answer.body.first().and_then(Value::as_str)?;
    Some(Taken::Request(handle.to_string()))
}

fn release(bus: &mut Bus, taken: Taken) {
    let message = match taken {
        Taken::Cookie((name, path, interface), cookie) => {
            Message::call(name, path, interface, "UnInhibit").with(vec![Value::U32(cookie)])
        }
        Taken::Request(handle) => Message::call(DESKTOP, &handle, REQUEST, "Close"),
    };
    let _ = bus.call(message, PATIENCE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_asked_until_something_is_held() {
        let mut awake = Awake::new("Test", "");
        awake.hold(Hold::NOTHING);
        assert!(awake.worker.is_none());
        assert_eq!(awake.held(), Hold::NOTHING);
    }

    #[test]
    fn the_holds_name_what_they_hold() {
        let holds = [
            Hold::NOTHING,
            Hold::SCREEN,
            Hold::SLEEP,
            Hold::SCREEN_AND_SLEEP,
        ];
        assert_eq!(
            holds.map(|hold| (hold.screen, hold.sleep, hold.any())),
            [
                (false, false, false),
                (true, false, true),
                (false, true, true),
                (true, true, true)
            ]
        );
    }
}
