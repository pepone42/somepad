use std::borrow::BorrowMut;
use std::default;

use floem::cosmic_text::{Attrs, AttrsList, FamilyOwned, LineHeightValue, TextLayout, Wrap};
use floem::id::Id;
use floem::kurbo::{Circle, Point, Rect};
use floem::peniko::{Brush, Color};
use floem::reactive::{create_effect, create_rw_signal, create_signal, ReadSignal, RwSignal};
use floem::unit::{PxPctAuto, UnitExt};
use floem::view::{View, ViewData};
use floem::views::{
    container, h_stack, label, rich_text, scroll, stack, v_stack, virtual_stack, Decorators,
    VirtualDirection, VirtualItemSize, VirtualVector,
};
use floem::widgets::button;
use floem::Renderer;
use ndoc::{Document, Rope};

struct TextEditor {
    data: ViewData,
    rope: Rope,
    text_node: Option<floem::taffy::prelude::Node>,
    viewport: Rect,
    line_height: f64,
}

pub fn text_editor(doc: impl Fn() -> Document + 'static) -> TextEditor {
    let id = Id::next();
    let attrs = Attrs::new()
        .family(&[FamilyOwned::Monospace])
        .font_size(14.);

    let mut t = TextLayout::new();
    t.set_text(" ", AttrsList::new(attrs));
    let line_height = (t.lines[0].layout_opt().as_ref().unwrap()[0].line_ascent
        + t.lines[0].layout_opt().as_ref().unwrap()[0].line_descent) as f64;
    TextEditor {
        data: ViewData::new(id),
        rope: doc().rope.clone(),
        text_node: None,
        viewport: Rect::default(),
        line_height,
    }
}

impl View for TextEditor {
    fn view_data(&self) -> &ViewData {
        &self.data
    }

    fn view_data_mut(&mut self) -> &mut ViewData {
        &mut self.data
    }

    fn layout(&mut self, cx: &mut floem::context::LayoutCx) -> floem::taffy::prelude::Node {
        cx.layout_node(self.id(), true, |cx| {
            let (width, height) = (1024., self.line_height * self.rope.len_lines() as f64); //attrs.line_height. * self.rope.len_lines());

            if self.text_node.is_none() {
                self.text_node = Some(
                    cx.app_state_mut()
                        .taffy
                        .new_leaf(floem::taffy::style::Style::DEFAULT)
                        .unwrap(),
                );
            }
            let text_node = self.text_node.unwrap();
            let style = floem::style::Style::new()
                .width(width)
                .height(height)
                .to_taffy_style();
            let _ = cx.app_state_mut().taffy.set_style(text_node, style);

            vec![text_node]
        })
    }

    fn compute_layout(&mut self, cx: &mut floem::context::ComputeLayoutCx) -> Option<Rect> {
        self.viewport = cx.current_viewport();

        None
    }

    fn paint(&mut self, cx: &mut floem::context::PaintCx) {
        let mut layout = TextLayout::new();
        let attrs = Attrs::new()
            .color(Color::BLACK)
            .family(&[FamilyOwned::Monospace])
            .font_size(14.);

        let attr_list = AttrsList::new(attrs);

        let mut y = 0.;
        for l in self.rope.lines() {
            layout.set_text(&l.to_string(), attr_list.clone());
            cx.draw_text(&layout, Point::new(0., y));
            y+= self.line_height;
        }

        // // let l = layout.lines
        // // .iter()
        // // .flat_map(|l| l.layout_opt().as_deref())
        // // .flat_map(|ls| ls.iter())
        // // .filter(|l| !l.glyphs.is_empty()).nth(0).unwrap().;

        // layout.set_text(&self.rope.line(0).to_string(), AttrsList::new(attrs));
        // cx.draw_text(&layout, Point::new(100., 100.));
        // // let c = Circle::new(Point::new(100., 100.), 50.);
        // let r = Rect::new(0.0, 0.0, 400.0, 400.0);
        // let b = Brush::Solid(Color::BLACK);
        // cx.stroke(&r, &b, 2.0);
    }
}

fn editor(doc: impl Fn() -> Document + 'static) -> impl View {
    let text_editor = text_editor(move || doc());

    container(
        scroll(stack((text_editor.on_click_stop(|e| {dbg!(e.point());}).style(|s| s.size_full()),)).style(|s| s.padding(6.0)))
            // .on_scroll(move |r| {
            //     vp_set.set(r);
            // })
            .style(|s| s.absolute().size_pct(100.0, 100.0)),
    )
    .style(move |s| s.border(1.0).border_radius(6.0).size_full())
}

fn app_view() -> impl View {
    let mut ndoc = ndoc::Document::from_file("src/main.rs").unwrap();

    // Create a reactive signal with a counter value, defaulting to 0
    let (indentation, set_indentation) = create_signal(ndoc.file_info.indentation);
    let (doc, set_doc) = create_signal(ndoc);
    let text_layout = create_rw_signal(TextLayout::new());

    create_effect(move |_| {
        let attrs = Attrs::new()
            .color(Color::BLACK)
            .family(&[FamilyOwned::Monospace])
            .font_size(18.);
        //.line_height(LineHeightValue::Normal(line_height));
        let attrs_list = AttrsList::new(attrs);

        let text = doc.get().rope.to_string();
        text_layout.update(|text_layout| {
            text_layout.set_text(&text, attrs_list);
            text_layout.set_tab_width(doc.get().file_info.indentation.len());
            text_layout.set_wrap(Wrap::None);
        });
    });

    create_effect(move |v| {
        set_doc.update(|d| d.file_info.indentation = indentation.get());
    });

    // Create a vertical layout
    v_stack((
        editor(move || doc.get()),
        // // The counter value updates automatically, thanks to reactivity
        // container(
        //     scroll(
        //         stack((
        //             //rich_text(move || text_layout.get())
        //             text_editor(move || doc.get())
        //                 // .on_resize(move |rect| {
        //                 //     text_area_rect.set(rect);
        //                 // })
        //                 .style(|s| s.size_full()),
        //             // label(|| " ".to_string()).style(move |s| {
        //             //     let cursor_pos = cursor_pos();
        //             //     s.absolute()
        //             //         .line_height(line_height)
        //             //         .margin_left(cursor_pos.x as f32 - 1.0)
        //             //         .margin_top(cursor_pos.y as f32)
        //             //         .border_left(2.0)
        //             //         .border_color(config.get().color(LapceColor::EDITOR_CARET))
        //             //         .apply_if(!is_active(), |s| s.hide())
        //             // }),
        //         ))
        //         .style(|s| s.padding(6.0)),
        //     )
        //     .on_scroll(|r| {dbg!(r);})
        //     .style(|s| s.absolute().size_pct(100.0, 100.0)),
        // )
        // .style(move |s| {
        //     //let config = config.get();
        //     s.border(1.0).border_radius(6.0).size_full()
        //     // .border_color(config.color(LapceColor::LAPCE_BORDER))
        //     // .background(config.color(LapceColor::EDITOR_BACKGROUND))
        // }),
        h_stack((
            label(|| "filename").style(|s| s.width_full().height(24.)),
            label(move || indentation.get().len()),
            label(move || doc.get().file_info.indentation.len()),
            label(move || doc.get().file_info.syntax),
            button(|| "change indent").on_click_stop(move |_| {
                set_indentation.set(ndoc::Indentation::Tab(8));
                set_doc.update(|d| d.insert("hahahahaha"));
            }),
            // button(|| "Decrement").on_click_stop(move |_| {
            //     set_counter.update(|value| *value -= 1);
            // }),
        )), //.style(|s| s.height(24.).min_height(24.)),
    ))
    .style(|s| s.flex_col().width_full().height_full().font_size(14.))
}

fn main() {
    floem::launch(app_view);
}
