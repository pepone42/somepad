use std::{cell::RefCell, collections::HashMap, default};

use floem::{
    action::open_file,
    event::Event,
    file::FileDialogOptions,
    id::Id,
    kurbo::{Rect, Size},
    reactive::{create_effect, create_rw_signal, RwSignal},
    view::{self, default_compute_layout, View, ViewData, Widget},
    views::{container, Container, Decorators},
};
use ndoc::Document;

use crate::{decorators::CustomDecorators, get_settings, settings::Settings};
use crate::{documents::Documents, shortcut};

use super::palette_list;

pub struct EditorWindow {
    data: ViewData,
    child: Box<dyn Widget>,
    pub viewport: RwSignal<Rect>,
}

impl View for EditorWindow {
    fn view_data(&self) -> &ViewData {
        &self.data
    }

    fn view_data_mut(&mut self) -> &mut ViewData {
        &mut self.data
    }

    fn build(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}

impl Widget for EditorWindow {
    fn view_data(&self) -> &ViewData {
        &self.data
    }

    fn view_data_mut(&mut self) -> &mut ViewData {
        &mut self.data
    }

    fn compute_layout(&mut self, cx: &mut floem::context::ComputeLayoutCx) -> Option<Rect> {
        self.viewport.set(cx.current_viewport());
        default_compute_layout(self, cx)
    }

    fn for_each_child<'a>(&'a self, for_each: &mut dyn FnMut(&'a dyn Widget) -> bool) {
        for_each(&self.child);
    }

    fn for_each_child_mut<'a>(&'a mut self, for_each: &mut dyn FnMut(&'a mut dyn Widget) -> bool) {
        for_each(&mut self.child);
    }

    fn for_each_child_rev_mut<'a>(
        &'a mut self,
        for_each: &mut dyn FnMut(&'a mut dyn Widget) -> bool,
    ) {
        for_each(&mut self.child);
    }

    fn debug_name(&self) -> std::borrow::Cow<'static, str> {
        "EditorWindow".into()
    }
}

thread_local! {
    pub static WINDOWS_VIEWPORT: RefCell<HashMap<Id, RwSignal<Rect>>> = RefCell::new(HashMap::new());
}

pub fn get_id_path(id: Id) -> Vec<Id> {
    let mut v = vec![id];
    let mut id = id;
    loop {
        if let Some(parent_id) = id.parent() {
            v.push(parent_id);
            id = parent_id;
        } else {
            break;
        }
    }
    v
}

pub trait Palette {
    fn palette(
        self,
        items: impl Iterator<Item = (usize, String)>,
        on_select: impl Fn(usize) + 'static + Clone + Copy,
    );
}

impl Palette for Id {
    fn palette(
        self,
        items: impl Iterator<Item = (usize, String)>,
        on_select: impl Fn(usize) + 'static + Clone + Copy,
    ) {
        palette_list(self, items, on_select);
    }
}

pub fn window<V: View + 'static>(child: V, documents: RwSignal<Documents>) -> EditorWindow {
    let w = EditorWindow {
        data: ViewData::new(Id::next()),
        child: child.build(),
        viewport: create_rw_signal(Default::default()),
    };

    let id = dbg!(w.id());

    let disabled = create_rw_signal(false);
    let viewport = create_rw_signal(Rect::new(0., 0., 100., 100.));

    WINDOWS_VIEWPORT.with(move |w| w.borrow_mut().insert(id, viewport.clone()));

    let w = w.disabled(move || disabled.get());

    let w = w.on_shortcut(shortcut!(Ctrl + n), move |_| {
        if disabled.get() {
            return;
        };
        let doc = create_rw_signal(Document::new(get_settings().indentation));
        documents.update(|d| {
            d.add(doc);
        });
    });

    let w = w.on_shortcut(shortcut!(Ctrl + o), move |_| {
        if disabled.get() {
            return;
        };
        disabled.set(true);
        let doc = create_rw_signal(Document::new(get_settings().indentation));
        open_file(FileDialogOptions::new().title("Open new file"), move |p| {
            if let Some(path) = p {
                doc.set(Document::from_file(&path.path[0]).unwrap());
                documents.update(|d| d.add(doc));
                disabled.set(false);
            }
        });
    });

    let w = w.on_event(floem::event::EventListener::WindowResized, move |s| {
        if let Event::WindowResized(s) = s {
            viewport.set(Rect::new(0., 0., s.width, s.height));
        }
        floem::EventPropagation::Continue
    });

    let w = w.on_shortcut(shortcut!(Ctrl + p), move |_| {
        if disabled.get() {
            return;
        };
        disabled.set(true);
        if !documents.get().is_empty() {
            id.palette(
                //viewport,
                documents
                    .get()
                    .order_by_mru()
                    .iter()
                    .enumerate()
                    .map(|(_, d)| (d.get().id(), d.get().title().to_string())),
                move |i| {
                    documents.update(|d| d.set_current(i));
                    disabled.set(false);
                },
            );
        }
    });

    let w = w.on_shortcut(shortcut!(Ctrl + w), move |_| {
        if disabled.get() {
            return;
        };
        documents.update(|d| d.remove(d.current_id()));
    });



    w
}
