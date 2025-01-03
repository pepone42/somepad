use std::collections::HashMap;

use cushy::context::LayoutContext;
use cushy::figures::Size;
use cushy::value::{Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetRef, WrapperWidget};
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
    pub editors: HashMap<usize, WidgetRef>,
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
                let editor = CodeEditor::new(d.clone(), cmd_reg.clone(), modal.clone())
                    .make_widget().into_ref();
                (d.get().id(), editor)
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
}

impl WrapperWidget for EditorSwitcher {
    fn child_mut(&mut self) -> &mut cushy::widget::WidgetRef {
        let id = self.documents.get()[self.current_doc.get()].get().id();

        if let std::collections::hash_map::Entry::Vacant(e) = self.editors.entry(id) {
            e.insert(
                CodeEditor::new(
                    self.documents.get()[self.current_doc.get()].clone(),
                    self.cmd_reg.clone(),
                    self.modal.clone(),
                )
                .make_widget()
                .into_ref());
        }
        let e = self.editors.get_mut(&id).unwrap();
        e
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
