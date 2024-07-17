mod document;
mod file_info;
mod rope_utils;
pub mod syntax;

pub use document::Document;
pub use file_info::FileInfo;
pub use file_info::LineFeed;
pub use file_info::Indentation;
pub use document::MoveDirection;
pub use document::Selection;
pub use document::SelectionAera;
pub use document::Position;
pub use ropey::Rope;
pub use syntect::highlighting::Color;
pub use syntect::highlighting::Theme as SyntectTheme;
