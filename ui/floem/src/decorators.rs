use floem::{event::Event, view::View, views::Decorators};

use crate::shortcut::Shortcut;

pub trait CustomDecorators: View + Sized {
    fn on_shortcut(self, shortcut: Shortcut, action: impl Fn(&Event) + 'static) -> Self {
        self.on_key_down(shortcut.key, shortcut.modifiers, action)
    }
}

impl<V: View> CustomDecorators for V {}
