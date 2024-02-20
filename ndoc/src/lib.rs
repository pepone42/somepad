mod document;
mod file_info;
pub mod rope_utils;

pub use document::Document;
pub use file_info::FileInfo;
pub use file_info::Indentation;
pub use document::MoveDirection;
pub use document::Selection;
pub use document::Position;
pub use ropey::Rope;
pub use document::position_to_char;
