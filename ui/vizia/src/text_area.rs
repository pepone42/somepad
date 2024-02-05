use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, rc::Rc, sync::{atomic::{AtomicBool, Ordering::Relaxed}, Arc, RwLock}};

use ndoc::{rope_utils, Document, Rope};
use once_cell::sync::Lazy;
use vizia::{binding::{Index, Lens, LensExt}, context::Context, vg::{Color, FontId, Paint, Path, TextContext}, view::{Handle, View}, views::Binding};

use crate::AppData;

pub struct TextArea;

impl TextArea {
    pub fn new(cx: &mut Context, lens: impl Lens<Target = Document>) -> Handle<Self> {
        Self{}.build(cx, |cx|{
            Binding::new(cx, lens, |cx,_| cx.needs_redraw());
        })
    }
}

impl View for TextArea {

    fn element(&self) -> Option<&'static str> {
        Some("text-area")
    }

    fn draw(&self, cx: &mut vizia::context::DrawContext, canvas: &mut vizia::view::Canvas) {
        
        static mut FONT_ID: Option<FontId> = None;

        let mut path = cx.build_path();
        cx.draw_shadows(canvas, &mut path);
        cx.draw_backdrop_filter(canvas, &mut path);
        cx.draw_background(canvas, &mut path);
        cx.draw_border(canvas, &mut path);
        cx.draw_inset_box_shadows(canvas, &mut path);
        cx.draw_outline(canvas);

        let rope = AppData::DOCUMENT.get(cx).rope;
        
        let fid = if unsafe{FONT_ID}.is_none() {
            let id = dbg!(canvas.add_font_mem(include_bytes!("../assets/Inconsolata.ttf")).unwrap());
            unsafe {FONT_ID = Some(id)};
            id
        } else {
            unsafe{FONT_ID}.unwrap()
        };
        //let ids: FontId = *Lazy::new(|| dbg!(canvas.add_font_mem(include_bytes!("../assets/Inconsolata.ttf")).unwrap()));
        let mut p = Paint::color(Color::white().into());
        p.set_font_size(18.0);
        p.set_font(&[fid]);
        p.set_anti_alias(false);
        

        let m = dbg!(canvas.measure_font(&p)).unwrap();
        
        let mut y = m.ascender();
        for line in rope.lines() {
            dbg!(canvas.fill_text(0., y, rope_utils::get_line_info(&line.slice(..),0 , 4),&p ));
            y+= m.height();
        }

        
        //dbg!(canvas.add_font_dir("assets"));

        //p.set_font(font_ids)
        
        // cosmic

        // canvas.draw_glyph_commands(draw_commands, &p, 1.0);

        let t = canvas.fill_text(100., 100., "hello",&p );
        
        let mut path = Path::new();
        path.circle(100., 100., 25.);
        canvas.stroke_path(&mut path, &p);
        
    }
}