use std::str::FromStr;

use floem::keyboard::{Key, ModifiersState};
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
    pub modifiers: ModifiersState,
}

#[macro_export]
macro_rules! shortcut {
    (Ctrl+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(stringify!($c))),
            modifiers: floem::keyboard::ModifiersState::CONTROL,
        }
    };
    (Ctrl+Shift+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c).to_uppercase(),
            )),
            modifiers: floem::keyboard::ModifiersState::CONTROL
                | floem::keyboard::ModifiersState::SHIFT,
        }
    };
    (Ctrl+Alt+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(stringify!($c))),
            modifiers: floem::keyboard::ModifiersState::CONTROL
                | floem::keyboard::ModifiersState::ALT,
        }
    };
    (Shift+Alt+$c:ident) => {
        crate::shortcut::Shortcut {
            key: floem::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c).to_uppercase(),
            )),
            modifiers: floem::keyboard::ModifiersState::ALT
                | floem::keyboard::ModifiersState::SHIFT,
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

fn modifier_state_from_str(s: &str) -> Result<ModifiersState, ParseError> {
    match s {
        "Ctrl" => Ok(ModifiersState::CONTROL),
        "Shift" => Ok(ModifiersState::SHIFT),
        "Alt" => Ok(ModifiersState::ALT),
        "Super" => Ok(ModifiersState::SUPER),
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
                let mut modifiers = ModifiersState::empty();
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
        let key = if modifiers.shift_key() {
            Key::Character(SmolStr::new(c.to_uppercase()))
        } else {
            Key::Character(SmolStr::new(c))
        };
        Ok(Shortcut { key, modifiers })
    }
}

mod test {
    use std::{fmt::Error, str::FromStr};

    use floem::keyboard::{Key, ModifiersState};
    use smol_str::SmolStr;

    use crate::shortcut::{ParseError, Shortcut};

    #[test]
    fn from_string() {
        assert_eq!(
            Shortcut::from_str("Ctrl+s").unwrap(),
            Shortcut {
                key: Key::Character(SmolStr::new("s")),
                modifiers: ModifiersState::CONTROL
            }
        )
    }

    #[test]
    fn from_string_with_shift() {
        assert_eq!(
            Shortcut::from_str("Ctrl+Shift+s").unwrap(),
            Shortcut {
                key: Key::Character(SmolStr::new("S")),
                modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT
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
                modifiers: ModifiersState::CONTROL
            }
        );
        assert_eq!(
            shortcut!(Ctrl + Shift + s),
            Shortcut {
                key: Key::Character(SmolStr::new("S")),
                modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT
            }
        );
        assert_eq!(
            shortcut!(Ctrl + Alt + s),
            Shortcut {
                key: Key::Character(SmolStr::new("s")),
                modifiers: ModifiersState::CONTROL | ModifiersState::ALT
            }
        );
    }
}
