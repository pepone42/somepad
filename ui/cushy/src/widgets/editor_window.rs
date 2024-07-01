use cushy::context::WidgetContext;
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::ModifiersState;
use cushy::value::{Dynamic, Source, Switchable};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetRef, WidgetTag, WrapperWidget,
    HANDLED, IGNORED,
};

use cushy::widgets::Custom;
use cushy::window::KeyEvent;

use ndoc::Document;

use crate::shortcut::event_match;
use crate::CommandsRegistry;

use super::editor_switcher::EditorSwitcher;
use super::opened_editor::OpenedEditor;
use super::palette::Palette;

#[derive(Debug)]
pub struct EditorWindow {
    child: WidgetRef,
    pub documents: Dynamic<Vec<Dynamic<Document>>>,
    pub current_doc: Dynamic<usize>,
    cmd_reg: Dynamic<CommandsRegistry>,
    focused: Dynamic<bool>,
}

impl EditorWindow {
    #[must_use]
    pub fn new(document: Dynamic<Document>, cmd_reg: Dynamic<CommandsRegistry>) -> Self {
        let documents = Dynamic::new(vec![document]);
        let current_doc = Dynamic::new(0);

        let (editor_tag, editor_id) = WidgetTag::new();
        let child = OpenedEditor::new(documents.clone(), current_doc.clone())
            .and(
                EditorSwitcher::new(documents.clone(), current_doc.clone(), cmd_reg.clone())
                    .make_with_tag(editor_tag),
            )
            .into_columns()
            .make_widget();

        let w = child
            .and(Palette::new().with_next_focus(editor_id))
            .into_layers();
        EditorWindow {
            child: w.widget_ref(),
            documents: documents.clone(),

            current_doc: current_doc.clone(),
            cmd_reg,
            focused: Dynamic::new(false),
        }
    }

    pub fn add_new_doc(&self, doc: Dynamic<Document>, _context: &mut WidgetContext) {
        self.documents.lock().push(doc);
        *self.current_doc.lock() += 1;
        dbg!(self.current_doc.get());
    }

    pub fn current_doc(&self) -> Dynamic<Document> {
        self.documents.get()[self.current_doc.get()].clone()
    }
}

impl WrapperWidget for EditorWindow {
    fn mounted(&mut self, context: &mut cushy::context::EventContext<'_>) {
        self.focused = context.window().focused().clone();
    }

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
        if !self.focused.get() {
            return HANDLED;
        }
        if input.state == ElementState::Pressed
            && context
                .modifiers()
                .state()
                .intersects(ModifiersState::CONTROL | ModifiersState::ALT | ModifiersState::SUPER)
        {
            let v = self.cmd_reg.get().window_shortcut;
            let id = context.widget.widget().id();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(id, self, context);
                    return HANDLED;
                }
            }
            return IGNORED;
        }
        IGNORED
    }

    // If I don't handle mouse down event here, the focus is stollen from the editor when I click in the opened editor widget
    fn hit_test(
        &mut self,
        _location: cushy::figures::Point<cushy::figures::units::Px>,
        _context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }
    fn mouse_down(
        &mut self,
        _location: cushy::figures::Point<cushy::figures::units::Px>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        _context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        dbg!("scroll mouse down");
        IGNORED
    }
}
