
use crate::TextEditor;

#[derive(Clone,Copy)]
pub struct ViewCommand {
    pub name: &'static str,
    pub action: fn(&TextEditor),
}
