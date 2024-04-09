use std::str::FromStr;

use floem::keyboard::{Key, Modifiers};
use smol_str::SmolStr;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    InvalidShortcut(String),
    InvalidKey(String),
    InvalidModifiers(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
}

#[macro_export]
macro_rules! shortcut {
    (Ctrl+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(stringify!($c))),
            modifiers: floem::keyboard::Modifiers::CONTROL,
        }
    };
    (Ctrl+Shift+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c).to_uppercase(),
            )),
            modifiers: floem::keyboard::Modifiers::CONTROL
                | floem::keyboard::Modifiers::SHIFT,
        }
    };
    (Ctrl+Alt+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(stringify!($c))),
            modifiers: floem::keyboard::Modifiers::CONTROL
                | floem::keyboard::Modifiers::ALT,
        }
    };
    (Shift+Alt+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c).to_uppercase(),
            )),
            modifiers: floem::keyboard::Modifiers::ALT
                | floem::keyboard::Modifiers::SHIFT,
        }
    };
}

pub fn event_match(event: &floem::event::Event, shortcut: Shortcut) -> bool {
    if let floem::event::Event::KeyDown(e) = event {
        e.key.logical_key == shortcut.key && e.modifiers == shortcut.modifiers
    } else {
        false
    }
}

fn modifier_state_from_str(s: &str) -> Result<Modifiers, ParseError> {
    match s {
        "Ctrl" => Ok(Modifiers::CONTROL),
        "Shift" => Ok(Modifiers::SHIFT),
        "Alt" => Ok(Modifiers::ALT),
        "Super" => Ok(Modifiers::META),
        _ => Err(ParseError::InvalidModifiers(s.to_string())),
    }
}

// Ex of shortcut string: "Ctrl+Shift+K" "Ctrl+o"
impl FromStr for Shortcut {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.contains('+') {
            return Err(ParseError::InvalidShortcut(s.to_string()));
        }
        let keys = s.split('+').collect::<Vec<&str>>();
        let (mut modifiers, c) = match keys.as_slice() {
            [modifierstr @ .., c] => {
                let mut modifiers = Modifiers::empty();
                for m in modifierstr {
                    modifiers = modifiers.union(modifier_state_from_str(m)?);
                }
                if c.is_empty() {
                    return Err(ParseError::InvalidKey(c.to_string()));
                }
                (modifiers, c)
            }
            _ => return Err(ParseError::InvalidShortcut(s.to_string())),
        };
        let key = if modifiers.shift() {
            Key::Character(SmolStr::new(c.to_uppercase()))
        } else {
            Key::Character(SmolStr::new(c))
        };
        Ok(Shortcut { key, modifiers })
    }
}

mod test {
    use std::{fmt::Error, str::FromStr};

    use floem::keyboard::{Key, Modifiers};
    use smol_str::SmolStr;

    use crate::shortcut::{ParseError, Shortcut};

    #[test]
    fn from_string() {
        assert_eq!(
            Shortcut::from_str("Ctrl+s").unwrap(),
            Shortcut {
                key: Key::Character(SmolStr::new("s")),
                modifiers: Modifiers::CONTROL
            }
        )
    }

    #[test]
    fn from_string_with_shift() {
        assert_eq!(
            Shortcut::from_str("Ctrl+Shift+s").unwrap(),
            Shortcut {
                key: Key::Character(SmolStr::new("S")),
                modifiers: Modifiers::CONTROL | Modifiers::SHIFT
            }
        )
    }

    #[test]
    fn bad_modifier() {
        assert_eq!(
            Shortcut::from_str("Crtl+s"),
            Err(ParseError::InvalidModifiers("Crtl".to_string()))
        )
    }

    #[test]
    fn from_macro() {
        assert_eq!(
            shortcut!(Ctrl + s),
            Shortcut {
                key: Key::Character(SmolStr::new("s")),
                modifiers: Modifiers::CONTROL
            }
        );
        assert_eq!(
            shortcut!(Ctrl + Shift + s),
            Shortcut {
                key: Key::Character(SmolStr::new("S")),
                modifiers: Modifiers::CONTROL | Modifiers::SHIFT
            }
        );
        assert_eq!(
            shortcut!(Ctrl + Alt + s),
            Shortcut {
                key: Key::Character(SmolStr::new("s")),
                modifiers: Modifiers::CONTROL | Modifiers::ALT
            }
        );
    }
}
