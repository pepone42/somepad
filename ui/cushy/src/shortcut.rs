use std::{fmt::Display, str::FromStr};

use cushy::kludgine::app::winit::{
    event::{ElementState, Modifiers},
    keyboard::{Key, ModifiersState, NamedKey},
};
use serde::{de::Visitor, Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Shortcut(String),
    Key(String),
    Modifiers(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: ModifiersState,
}

impl Serialize for Shortcut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut mods = Vec::new();

        if self.modifiers.contains(ModifiersState::ALT) {
            mods.push("Alt");
        }
        if self.modifiers.contains(ModifiersState::CONTROL) {
            mods.push("Ctrl");
        }
        if self.modifiers.contains(ModifiersState::SHIFT) {
            mods.push("Shift");
        }
        if self.modifiers.contains(ModifiersState::SUPER) {
            mods.push("Meta");
        }

        if let Key::Character(c) = &self.key {
            serializer.serialize_str(&format!("{}+{}", mods.join("+"), c))
        } else {
            Err(serde::ser::Error::custom("Unsupported Key format"))
        }
    }
}

struct ShortcutVisitor;

impl<'de> Visitor<'de> for ShortcutVisitor {
    type Value = Shortcut;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A shorcut representation like 'Ctrl+c'")
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Shortcut::from_str(value).map_err(|e| serde::de::Error::custom(format!("{:?}", e)))
    }
}

impl<'de> Deserialize<'de> for Shortcut {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(ShortcutVisitor)
    }
}

#[macro_export]
macro_rules! shortcut {
    (Ctrl+Tab) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Named(cushy::kludgine::app::winit::keyboard::NamedKey::Tab),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL,
                
        }
    };
    (Ctrl+Shift+Tab) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Named(cushy::kludgine::app::winit::keyboard::NamedKey::Tab),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL
            | cushy::kludgine::app::winit::keyboard::ModifiersState::SHIFT,
        }
    };
    (Ctrl+ArrowDown) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Named(cushy::kludgine::app::winit::keyboard::NamedKey::ArrowDown),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL,
                
        }
    };
    (Ctrl+Alt+ArrowDown) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Named(cushy::kludgine::app::winit::keyboard::NamedKey::ArrowDown),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL
            | cushy::kludgine::app::winit::keyboard::ModifiersState::ALT,
        }
    };
    (Ctrl+ArrowUp) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Named(cushy::kludgine::app::winit::keyboard::NamedKey::ArrowUp),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL,
                
        }
    };
    (Ctrl+Alt+ArrowUp) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Named(cushy::kludgine::app::winit::keyboard::NamedKey::ArrowUp),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL
            | cushy::kludgine::app::winit::keyboard::ModifiersState::ALT,
        }
    };
    (Ctrl+$c:ident) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c),
            )),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL,
        }
    };
    (Ctrl+Shift+$c:ident) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c).to_uppercase(),
            )),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL
                | cushy::kludgine::app::winit::keyboard::ModifiersState::SHIFT,
        }
    };
    (Ctrl+Alt+$c:ident) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c),
            )),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::CONTROL
                | cushy::kludgine::app::winit::keyboard::ModifiersState::ALT,
        }
    };
    (Alt+$c:ident) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c),
            )),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::ALT,
        }
    };
    (Shift+Alt+$c:ident) => {
        $crate::shortcut::Shortcut {
            key: cushy::kludgine::app::winit::keyboard::Key::Character(smol_str::SmolStr::new(
                stringify!($c).to_uppercase(),
            )),
            modifiers: cushy::kludgine::app::winit::keyboard::ModifiersState::ALT
                | cushy::kludgine::app::winit::keyboard::ModifiersState::SHIFT,
        }
    };
}

pub fn event_match(
    input: &cushy::window::KeyEvent,
    modifiers: Modifiers,
    shortcut: Shortcut,
) -> bool {
    input.logical_key == shortcut.key && modifiers.state() == shortcut.modifiers && input.state == ElementState::Pressed
}

fn modifier_state_from_str(s: &str) -> Result<ModifiersState, ParseError> {
    match s {
        "Ctrl" => Ok(ModifiersState::CONTROL),
        "Shift" => Ok(ModifiersState::SHIFT),
        "Alt" => Ok(ModifiersState::ALT),
        "Super" => Ok(ModifiersState::SUPER),
        _ => Err(ParseError::Modifiers(s.to_string())),
    }
}

// Ex of shortcut string: "Ctrl+Shift+K" "Ctrl+o"
impl FromStr for Shortcut {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.contains('+') {
            return Err(ParseError::Shortcut(s.to_string()));
        }
        let keys = s.split('+').collect::<Vec<&str>>();
        let (modifiers, c) = match keys.as_slice() {
            [modifierstr @ .., c] => {
                let mut modifiers = ModifiersState::empty();
                for m in modifierstr {
                    modifiers = modifiers.union(modifier_state_from_str(m)?);
                }
                if c.is_empty() {
                    return Err(ParseError::Key(c.to_string()));
                }
                (modifiers, c)
            }
            _ => return Err(ParseError::Shortcut(s.to_string())),
        };
        let key = if modifiers.shift_key() {
            Key::Character(SmolStr::new(c.to_uppercase()))
        } else {
            Key::Character(SmolStr::new(c))
        };
        Ok(Shortcut { key, modifiers })
    }
}

impl Display for Shortcut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut mods = Vec::new();

        if self.modifiers.contains(ModifiersState::ALT) {
            mods.push("Alt");
        }
        if self.modifiers.contains(ModifiersState::CONTROL) {
            mods.push("Ctrl");
        }
        if self.modifiers.contains(ModifiersState::SHIFT) {
            mods.push("Shift");
        }
        if self.modifiers.contains(ModifiersState::SUPER) {
            mods.push("Meta");
        }

        match &self.key {
            Key::Character(c) => write!(f, "{}+{}", mods.join("+"), c),
            Key::Named(NamedKey::ArrowUp) => write!(f, "{}+Up", mods.join("+") ),
            Key::Named(NamedKey::ArrowDown) => write!(f, "{}+Down", mods.join("+") ),
            Key::Named(NamedKey::Tab) => write!(f, "{}+Tab", mods.join("+") ),
            _ => Err(std::fmt::Error),
        }

    }

}

#[allow(dead_code)]
pub trait ModifiersCustomExt {
    fn ctrl(&self) -> bool;
    fn shift(&self) -> bool;
    fn alt(&self) -> bool;
    fn meta(&self) -> bool;
    fn ctrl_alt(&self) -> bool;
    fn ctrl_shift(&self) -> bool;
    fn ctrl_shift_alt(&self) -> bool;
}

impl ModifiersCustomExt for Modifiers {
    fn ctrl(&self) -> bool {
        self.state().contains(ModifiersState::CONTROL)
    }

    fn shift(&self) -> bool {
        self.state().contains(ModifiersState::SHIFT)
    }

    fn alt(&self) -> bool {
        self.state().contains(ModifiersState::ALT)
    }

    fn meta(&self) -> bool {
        self.state().contains(ModifiersState::SUPER)
    }

    fn ctrl_alt(&self) -> bool {
        self.state().contains(ModifiersState::CONTROL | ModifiersState::ALT)
    }

    fn ctrl_shift(&self) -> bool {
        self.state().contains(ModifiersState::CONTROL | ModifiersState::SHIFT)
    }

    fn ctrl_shift_alt(&self) -> bool {
        self.state().contains(ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT)
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use cushy::kludgine::app::winit::keyboard::{Key, ModifiersState};
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
            Err(ParseError::Modifiers("Crtl".to_string()))
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
