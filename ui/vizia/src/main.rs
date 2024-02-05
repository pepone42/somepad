mod text_area;

use ndoc::{Document, FileInfo, Rope};
use vizia::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct AppData {
    document: ndoc::Document,
}

impl Model for AppData {}

pub mod appdat_derived_menses {

    #[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
    pub struct Document();

    #[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
    pub struct FileInfo();

    #[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
    pub struct Syntax();
}

impl Lens for appdat_derived_menses::Document {
    type Source = AppData;

    type Target = Document;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.document))
    }
}

impl Lens for appdat_derived_menses::FileInfo {
    type Source = AppData;

    type Target = FileInfo;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.document.file_info))
    }
}

impl Lens for appdat_derived_menses::Syntax {
    type Source = AppData;

    type Target = String;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.document.file_info.syntax))
    }
}

impl AppData {
    pub const DOCUMENT: Wrapper<appdat_derived_menses::Document> =
        Wrapper(appdat_derived_menses::Document());
    pub const FILE_INFO: Wrapper<appdat_derived_menses::FileInfo> =
        Wrapper(appdat_derived_menses::FileInfo());
    pub const SYNTAX: Wrapper<appdat_derived_menses::Syntax> =
        Wrapper(appdat_derived_menses::Syntax());
}

fn main() {
    let state = AppData {
        document: Document::from_file("cargo.toml").unwrap(),
    };
    let content = state.document.rope.to_string();

    Application::new(move |cx| {
        //cx.add_font_mem(include_bytes!("../assets/Inconsolata.ttf"));

        state.build(cx);

        cx.add_stylesheet(".editor {font-family : Consolas; font-weight: Bold; background-color: #343D46; color: #FEFEFE}")
            .unwrap();

        VStack::new(cx, |cx| {
            text_area::TextArea::new(cx, AppData::DOCUMENT).class("editor").size(Stretch(1.0));
            HStack::new(cx, |cx| {
                Label::new(cx, "Hello world!");//.class("mono");
                Label::new(cx, AppData::SYNTAX);//.class("mono");
            })
            .size(Auto);
        })
        .child_space(Pixels(1.0));
    })
    .title("xncode")
    .run();
}
