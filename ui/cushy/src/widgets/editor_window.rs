use cushy::kludgine::app::winit::event::ElementState;
use cushy::value::{Dynamic, Source, Switchable};
use cushy::widget::{
    EventHandling, MakeWidget, WidgetRef, WrapperWidget, HANDLED, IGNORED,
};

use cushy::widgets::Custom;
use cushy::window::KeyEvent;
use cushy::ModifiersExt;

use ndoc::Document;

use crate::shortcut::event_match;
use crate::WINDOW_SHORTCUT;

use super::palette::{Palette, PALETTE_STATE};

#[derive(Debug)]
pub struct EditorWindow {
    child: WidgetRef,
    documents: Dynamic<Vec<Dynamic<Document>>>,
}

impl EditorWindow {
    #[must_use]
    pub fn new(child: impl MakeWidget) -> impl MakeWidget {
        let palette = PALETTE_STATE.map_each(|p| p.active());// super::palette::PALETTE.clone();
        let enabled = palette.map_each(|p| !*p);

        let child = child.make_widget();
        let child_id = child.id();

        let w = child
            .with_enabled(enabled)
            .and(palette.clone().switcher(move |current, _active| {
                if *current {
                    Palette::new().make_widget()
                } else {
                    Custom::empty()
                        .on_mounted(move |c| c.for_other(&child_id).unwrap().focus())
                        .make_widget()
                }
            }))
            .into_layers();
        EditorWindow {
            child: w.widget_ref(),
            documents: Dynamic::new(Vec::new()),
        }
    }

    pub fn add_new_doc(&self, doc: Dynamic<Document>) {
        self.documents.lock().push(doc);
    }
}

impl WrapperWidget for EditorWindow {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn keyboard_input(
        &mut self,
        _device_id: cushy::window::DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        if input.state == ElementState::Pressed && context.modifiers().possible_shortcut() {
            let v = WINDOW_SHORTCUT.lock().unwrap();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(self);
                    return HANDLED;
                }
            }
            return IGNORED;
        }
        IGNORED
    }
}
