use std::collections::HashMap;
use std::vec;

use cushy::context::{AsEventContext, WidgetContext};
use cushy::figures::units::Lp;
use cushy::figures::IntoSigned;
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::ModifiersState;
use cushy::kludgine::app::winit::platform::windows::WindowExtWindows;
use cushy::kludgine::wgpu::rwh::HasWindowHandle;
use cushy::value::{Dynamic, Source, Switchable};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, WidgetId, WidgetInstance, WidgetList, WidgetRef,
    WidgetTag, WrapperWidget, HANDLED, IGNORED,
};

use cushy::widgets::{Custom, Switcher};
use cushy::window::KeyEvent;
use cushy::ModifiersExt;

use ndoc::Document;

use crate::shortcut::event_match;
use crate::CommandsRegistry;

use super::editor_switcher::EditorSwitcher;
use super::opened_editor::OpenedEditor;
use super::palette::{Palette, PALETTE_STATE};
use super::scroll::ScrollController;
use super::text_editor::CodeEditor;

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
        let palette = PALETTE_STATE.map_each(|p| p.active()); // super::palette::PALETTE.clone();
        let enabled = palette.map_each(|p| !*p);

        let documents = Dynamic::new(vec![document]);
        let current_doc = Dynamic::new(0);

        //let child = child.make_widget();

        
        

        // let docs = documents.clone();
        // let cregs = cmd_reg.clone();
        let (editor_tag, editor_id) = WidgetTag::new();
        // let editors = Dynamic::new(HashMap::new());
        // let editors_clone = editors.clone();
        let child = OpenedEditor::new(documents.clone(), current_doc.clone())
            //.width(Lp::new(50))
            .and(
                // current_doc
                //     .with_clone(move |current_doc| {
                        // current_doc.switcher(move |current, active| {
                        //     let cregs = cregs.clone();
                        //     let doc = docs.clone().get()[*current].clone();
                        //     editors_clone
                        //         .lock()
                        //         .entry(doc.get().id())
                        //         .or_insert_with(move || {
                        //             CodeEditor::new(doc.clone(), cregs.clone()).make_widget()
                        //         })
                        //         .clone()
                        // })
                        EditorSwitcher::new(documents.clone(),current_doc.clone(), cmd_reg.clone())
                            
                    //})
                    .make_with_tag(editor_tag),
            )
            .into_columns()
            .make_widget();

        let w = child
            .with_enabled(enabled)
            .and(palette.clone().switcher(move |current, _active| {
                if *current {
                    Palette::new().make_widget()
                } else {
                    Custom::empty()
                        .on_mounted(move |c| c.for_other(&editor_id).unwrap().focus())
                        .make_widget()
                }
            }))
            .into_layers();
        EditorWindow {
            child: w.widget_ref(),
            documents: documents.clone(),

            current_doc: current_doc.clone(),
            cmd_reg,
            focused: Dynamic::new(false),
        }
    }

    pub fn add_new_doc(&self, doc: Dynamic<Document>, context: &mut WidgetContext) {


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
}
