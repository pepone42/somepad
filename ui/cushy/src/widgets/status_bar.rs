use cushy::{
    figures::{units::Px, Point, ScreenScale, Size, Zero}, kludgine::{text::Text, DrawableExt}, styles::{components::{FontFamily, LineHeight}, Dimension}, value::{Dynamic, Source}, widget::Widget
};
use ndoc::{Document, Indentation};

#[derive(Debug)]
pub struct StatusBar {
    documents: Dynamic<Vec<Dynamic<Document>>>,
    current_doc: Dynamic<usize>,
}

impl StatusBar {
    pub fn new(documents: Dynamic<Vec<Dynamic<Document>>>, current_doc: Dynamic<usize>) -> Self {
        StatusBar {
            documents: documents.clone(),
            current_doc: current_doc.clone(),
        }
    }
}

impl Widget for StatusBar {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        context.apply_current_font_settings();

        let font_familly= context.get(&FontFamily);
        if let Some(font_familly) = context.find_available_font_family(&font_familly) {
            context.gfx.set_font_family(font_familly);
        }

        let doc = self.documents.get()[self.current_doc.get()].clone();
        let doc = doc.get();
        let dirty = if doc.is_dirty() { "*" } else { "" };
        let s = context.gfx.region();
        

        let selection = if doc.selections.len()>1 {
            format!("{} selections", doc.selections.len())
        } else {
            format!("Ln {}, Col {}", doc.selections[0].head.line + 1, doc.selections[0].head.column + 1)
        };
        

        let indent = match doc.file_info.indentation {
            Indentation::Space(s) => format!("Spaces: {}", s),
            Indentation::Tab(t) => format!("Tabs: {}", t),
        };

        let eol = match doc.file_info.linefeed {
            ndoc::LineFeed::CR => "CR",
            ndoc::LineFeed::LF => "LF",
            ndoc::LineFeed::CRLF => "CRLF",
        };

        let file_name = if let Some(file_name) = doc.file_name {
            file_name.file_name().unwrap().to_string_lossy().to_string()
        } else {
            "Untitled".to_string()
        };
        let text = format!("{}{}", file_name, dirty);
        let text = Text::<Px>::new(&text, cushy::kludgine::Color::WHITE);
        

        context.gfx.draw_text(text.translate_by(Point::ZERO));
        let text = format!("{}  {}  {}  {}  {} ", selection, indent, eol , doc.file_info.encoding.name(), doc.file_info.syntax.name);
        let text = Text::<Px>::new(&text, cushy::kludgine::Color::WHITE);
        let mtext = context.gfx.measure_text(text);
        context.gfx.draw_text(text.translate_by(Point::new( s.size.width - mtext.size.width, Px::ZERO)));

    }
    fn layout(
            &mut self,
            available_space: cushy::figures::Size<cushy::ConstraintLimit>,
            context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
        ) -> cushy::figures::Size<cushy::figures::units::UPx> {
            let heigh = match context.get(&LineHeight) {
                Dimension::Lp(v) => v.into_upx(context.gfx.scale()),
                Dimension::Px(v) => v.into_upx(context.gfx.scale()),
            };
            
            // if I don't substract 1 from the width, the layout/redraw is called infinitely, switching between max()+1 and max() 
        Size::new(available_space.width.max()-1 , heigh)
    }
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut cushy::context::EventContext<'_>) -> bool {
        true
    }
}
