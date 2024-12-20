use cushy::{
    value::{Destination, Dynamic, ForEach, Source},
    widget::MakeWidget,
    widgets::Space,
};
use ndoc::{Document, Indentation};

#[derive(Debug)]
pub struct StatusBar {
    filename: Dynamic<String>,
    selection: Dynamic<String>,
    indent: Dynamic<String>,
    eol: Dynamic<String>,
    encoding: Dynamic<String>,
    syntax: Dynamic<String>,
}

impl StatusBar {
    pub fn new(documents: Dynamic<Vec<Dynamic<Document>>>, current_doc: Dynamic<usize>) -> Self {
        let filename = Dynamic::new(String::new());
        let selection = Dynamic::new(String::new());
        let indent = Dynamic::new(String::new());
        let eol = Dynamic::new(String::new());
        let encoding = Dynamic::new(String::new());
        let syntax = Dynamic::new(String::new());

        (&documents, &current_doc)
            .for_each({
                let filename = filename.clone();
                let selection = selection.clone();
                let indent = indent.clone();
                let eol = eol.clone();
                let encoding = encoding.clone();
                let syntax = syntax.clone();
                move |(d, c)| {
                    let filename = filename.clone();
                    let selection = selection.clone();
                    let indent = indent.clone();
                    let eol = eol.clone();
                    let encoding = encoding.clone();
                    let syntax = syntax.clone();
                    if let Some(doc) = d.get(*c) {
                        doc.for_each(move |doc| {
                            filename.replace(format!(
                                "{}{}",
                                doc.file_name
                                    .as_ref()
                                    .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
                                    .unwrap_or("Untitled".to_string()),
                                if doc.is_dirty() { "*" } else { "" }
                            ));
                            selection.replace(if doc.selections.len() > 1 {
                                format!("{} selections", doc.selections.len())
                            } else {
                                format!(
                                    "Ln {}, Col {}",
                                    doc.selections[0].head.line + 1,
                                    doc.selections[0].head.column + 1
                                )
                            });
                            indent.replace(match doc.file_info.indentation {
                                Indentation::Space(s) => format!("Spaces: {}", s),
                                Indentation::Tab(t) => format!("Tabs: {}", t),
                            });
                            eol.replace(match doc.file_info.linefeed {
                                ndoc::LineFeed::CR => "CR".to_string(),
                                ndoc::LineFeed::LF => "LF".to_string(),
                                ndoc::LineFeed::CRLF => "CRLF".to_string(),
                            });
                            encoding.replace(doc.file_info.encoding.name().to_string());
                            syntax.replace(doc.file_info.syntax.name.clone());
                        })
                        .persist();
                    }
                }
            })
            .persist();

        StatusBar {
            filename,
            selection,
            indent,
            eol,
            encoding,
            syntax,
        }
    }
}

impl MakeWidget for StatusBar {
    fn make_widget(self) -> cushy::widget::WidgetInstance {
        self.filename
            .and(Space::clear().expand())
            .and(self.selection)
            .and(self.indent)
            .and(self.eol)
            .and(self.encoding)
            .and(self.syntax)
            .into_columns()
            .make_widget()
    }
}
