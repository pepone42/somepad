
use crate::TextEditor;

pub struct ViewCommand {
    pub name: &'static str,
    pub action: fn(&mut TextEditor),
}
