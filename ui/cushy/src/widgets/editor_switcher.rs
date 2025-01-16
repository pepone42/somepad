use std::collections::HashMap;

use cushy::context::LayoutContext;
use cushy::figures::Size;
use cushy::value::{Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetInstance, WidgetRef, WrapperWidget};
use cushy::widgets::layers::Modal;
use cushy::ConstraintLimit;
use ndoc::Document;

use crate::CommandsRegistry;

use super::text_editor::CodeEditor;

#[derive(Debug)]
pub struct EditorSwitcher {
    pub(super) documents: Dynamic<Vec<Dynamic<Document>>>,
    pub(super) current_doc: Dynamic<usize>,

    last_doc: usize,
    pub editors: HashMap<usize, (WidgetRef, WidgetInstance)>,
    cmd_reg: Dynamic<CommandsRegistry>,
    modal: Modal,
}

impl EditorSwitcher {
    pub fn new(
        documents: Dynamic<Vec<Dynamic<Document>>>,
        current_doc: Dynamic<usize>,
        cmd_reg: Dynamic<CommandsRegistry>,
        modal: Modal,
    ) -> Self {
        let editors = documents
            .get()
            .iter()
            .map(|d| {
                let code_editor = CodeEditor::new(d.clone(), cmd_reg.clone(), modal.clone());
                let editor_instance = code_editor.editor.clone();
                
                let editor_ref = code_editor.make_widget().into_ref();
                (d.get().id(), (editor_ref, editor_instance))
            })
            .collect();

        EditorSwitcher {
            documents,
            editors,
            current_doc,
            last_doc: 0,
            cmd_reg,
            modal,
        }
    }

    pub fn current_editor(&self) -> WidgetInstance {
        self.editors.get(&self.current_doc.get()).expect("a valid current document id").1.clone()
    }
}

impl WrapperWidget for EditorSwitcher {
    fn child_mut(&mut self) -> &mut cushy::widget::WidgetRef {
        let id = self.documents.get()[self.current_doc.get()].get().id();

        if let std::collections::hash_map::Entry::Vacant(e) = self.editors.entry(id) {
            let editor_instance = CodeEditor::new(
                self.documents.get()[self.current_doc.get()].clone(),
                self.cmd_reg.clone(),
                self.modal.clone(),
            )
            .make_widget();
            let editor_ref = editor_instance.clone().into_ref();
            e.insert((editor_ref, editor_instance));
        }
        let e = self.editors.get_mut(&id).unwrap();
        &mut e.0
    }

    fn adjust_child_constraints(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        context.invalidate_when_changed(&self.current_doc);
        // TODO: when a doc is close, we should remove the editor from the hashmap
        if self.last_doc != self.current_doc.get() {
            self.child_mut().mount_if_needed(context);
            let current_widget_id = self.child_mut().widget().id();

            // TODO: What if the previous editor was not focused?
            context.for_other(&current_widget_id).unwrap().focus();

            self.last_doc = self.current_doc.get();
        }
        available_space
    }
}
