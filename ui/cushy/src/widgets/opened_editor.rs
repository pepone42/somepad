use cushy::{figures::{units::{Px, UPx}, Point, Size, Zero}, kludgine::{app::winit::platform::windows::Color, text::Text, DrawableExt}, value::{Dynamic, Source}, widget::Widget};
use ndoc::Document;

#[derive(Debug)]
pub struct OpenedEditor {
    documents: Dynamic<Vec<Dynamic<Document>>>,
    current_doc: Dynamic<usize>,
}

impl OpenedEditor {
    pub fn new(documents: Dynamic<Vec<Dynamic<Document>>>, current_doc: Dynamic<usize>) -> Self {
        OpenedEditor {
            documents,
            current_doc,
        }
    }
}

impl Widget for OpenedEditor {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let mut y = Px::ZERO;
        for (i, doc) in self.documents.get().iter().enumerate() {
            let mut text = if let Some(file_name) = doc.get().file_name {
                file_name.file_name().unwrap().to_string_lossy().to_string()
            } else {
                format!("Untitled {}", i)
            };
            if i == self.current_doc.get() {
                text.push_str(" (current)");
            }
            let text = Text::new(&text, cushy::kludgine::Color::WHITE);
            context.gfx.draw_text(text.translate_by(Point::new(Px::ZERO, y)));
            y += 20;
        }

    }

    fn layout(
            &mut self,
            available_space: cushy::figures::Size<cushy::ConstraintLimit>,
            context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
        ) -> cushy::figures::Size<cushy::figures::units::UPx> {
        let h = UPx::new(self.documents.get().len() as _) * 20;
        Size::new(UPx::new(100), h)
    }
}