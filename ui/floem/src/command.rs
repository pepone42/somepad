
use crate::{widgets::EditorWindow, TextEditor};

#[derive(Clone,Copy)]
pub struct ViewCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(&TextEditor),
}

#[derive(Clone,Copy)]
pub struct WindowCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(&EditorWindow),
}
