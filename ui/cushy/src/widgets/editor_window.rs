use std::collections::HashMap;
use std::time::SystemTime;

use cushy::context::WidgetContext;
use cushy::figures::units::Px;
use cushy::figures::Zero;
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::ModifiersState;
use cushy::value::{Dynamic, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, WidgetRef, WidgetTag, WrapperWidget, HANDLED,
    IGNORED,
};

use cushy::window::KeyEvent;

use ndoc::Document;

use crate::shortcut::event_match;
use crate::CommandsRegistry;

use super::editor_switcher::EditorSwitcher;
use super::opened_editor::{OpenedEditor, ResizeHandle};
use super::palette::Palette;
use super::scroll::MyScroll;
use super::side_bar::SideBar;

#[derive(Debug)]
pub struct EditorWindow {
    child: WidgetRef,
    pub documents: Dynamic<Vec<Dynamic<Document>>>,
    pub current_doc: Dynamic<usize>,
    pub cmd_reg: Dynamic<CommandsRegistry>,
    pub mru_documents: Dynamic<HashMap<usize, SystemTime>>,
    focused: Dynamic<bool>,
}

impl EditorWindow {
    #[must_use]
    pub fn new(document: Dynamic<Document>, cmd_reg: Dynamic<CommandsRegistry>) -> Self {
        let documents = Dynamic::new(vec![document]);
        let current_doc = Dynamic::new(0);
        let lru = Dynamic::new(HashMap::new());
        lru.lock().insert(0, SystemTime::now());
        let h = lru.with_clone(|lru| {
            current_doc.for_each(move |current_doc| {
                *lru.lock().entry(*current_doc).or_insert(SystemTime::now()) = SystemTime::now();
            })
        });
        h.persist();
        let (editor_tag, editor_id) = WidgetTag::new();
        // TODO: Use Lp instead of Px
        let width = Dynamic::new(Px::new(200));
        let opened_editor = SideBar::new(OpenedEditor::new(documents.clone(), current_doc.clone()),width.clone());
        
        let child = MyScroll::vertical(opened_editor).expand_vertically()
            .and(ResizeHandle::new(width)).and(
                EditorSwitcher::new(documents.clone(), current_doc.clone(), cmd_reg.clone())
                    .make_with_tag(editor_tag),
            )
            .into_columns().gutter(Px::ZERO)
            .make_widget();

        let w = child
            .and(Palette::new().with_next_focus(editor_id))
            .into_layers();
        EditorWindow {
            child: w.widget_ref(),
            documents: documents.clone(),
            mru_documents: lru,
            current_doc: current_doc.clone(),
            cmd_reg,
            focused: Dynamic::new(false),
        }
    }

    pub fn add_new_doc(&self, doc: Dynamic<Document>, _context: &mut WidgetContext) {
        self.documents.lock().push(doc);
        *self.current_doc.lock() += 1;
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
        IGNORED
    }
}
