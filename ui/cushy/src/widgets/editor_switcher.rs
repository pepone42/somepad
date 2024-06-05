use std::collections::HashMap;

use cushy::context::LayoutContext;
use cushy::figures::Size;
use cushy::value::{Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetRef, WrapperWidget, IGNORED};
use cushy::ConstraintLimit;
use ndoc::Document;

use crate::CommandsRegistry;

use super::text_editor::CodeEditor;

#[derive(Debug)]
pub struct EditorSwitcher {
    pub(super) documents: Dynamic<Vec<Dynamic<Document>>>,
    pub(super) current_doc: Dynamic<usize>,

    last_doc: usize,
    editors: HashMap<usize, WidgetRef>,
    cmd_reg: Dynamic<CommandsRegistry>,
}

impl EditorSwitcher {
    pub fn new(
        documents: Dynamic<Vec<Dynamic<Document>>>,
        current_doc: Dynamic<usize>,
        cmd_reg: Dynamic<CommandsRegistry>,
    ) -> Self {
        let editors = documents
            .get()
            .iter()
            .map(|d| {
                let editor = CodeEditor::new(d.clone(), cmd_reg.clone())
                    .make_widget()
                    .widget_ref();
                (d.get().id(), editor)
            })
            .collect();

        EditorSwitcher {
            documents,
            editors,
            current_doc,
            last_doc: 0,
            cmd_reg,
        }
    }
}

impl WrapperWidget for EditorSwitcher {
    fn child_mut(&mut self) -> &mut cushy::widget::WidgetRef {
        //dbg!(self.last_doc,self.current_doc.get());

        let id = self.documents.get()[self.current_doc.get()].get().id();
        if !self.editors.contains_key(&id) {
            self.editors.insert(
                id,
                CodeEditor::new(
                    self.documents.get()[self.current_doc.get()].clone(),
                    self.cmd_reg.clone(),
                )
                .make_widget()
                .widget_ref(),
            );
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
            dbg!("switch!");

            self.child_mut().mount_if_needed(context);
            let current_widget_id = self.child_mut().widget().id();

            // TODO: What if the previous editor was not focused?
            context.for_other(&current_widget_id).unwrap().focus();

            self.last_doc = self.current_doc.get();
        }
        available_space
    }
}
