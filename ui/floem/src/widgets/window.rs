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
    EventPropagation,
};
use ndoc::Document;

use crate::{
    decorators::CustomDecorators, get_settings, settings::Settings, shortcut::event_match,
    WINDOW_COMMAND_REGISTRY, WINDOW_SHORTCUT,
};
use crate::{documents::Documents, shortcut};

pub enum WindowUpdateCommand {
    LaunchCommand(String),
}

pub struct EditorWindow {
    data: ViewData,
    child: Box<dyn Widget>,
    pub viewport: RwSignal<Rect>,
    pub documents: RwSignal<Documents>,
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

    fn update(&mut self, _cx: &mut floem::context::UpdateCx, state: Box<dyn std::any::Any>) {
        if let Some(state) = state.downcast_ref::<WindowUpdateCommand>() {
            match state {
                WindowUpdateCommand::LaunchCommand(cmdid) => WINDOW_COMMAND_REGISTRY.with(|registry| {
                    dbg!(&cmdid);
                    if let Some(cmd) = registry.borrow().get(cmdid.as_str()) {
                        (cmd.action)(self);
                    }
                }),
            }
        }
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

// pub trait Palette {
//     fn palette(
//         self,
//         items: impl Iterator<Item = (usize, String)>,
//         on_select: impl Fn(usize) + 'static + Clone + Copy,
//     );
// }

// impl Palette for Id {
//     fn palette<usize>(
//         self,
//         items: impl Iterator<Item = (usize, String)>,
//         on_select: impl FnOnceCopyable<usize>,
//     ) {
//         palette_list(self, items, Box::new(on_select));
//     }
// }

pub fn window<V: View + 'static>(child: V, documents: RwSignal<Documents>) -> EditorWindow {
    let w = EditorWindow {
        data: ViewData::new(Id::next()),
        child: child.build(),
        viewport: create_rw_signal(Default::default()),
        documents,
    }; //.keyboard_navigatable();

    let id = dbg!(w.id());

    let disabled = create_rw_signal(false);
    let viewport = create_rw_signal(Rect::new(0., 0., 100., 100.));

    WINDOWS_VIEWPORT.with(move |w| w.borrow_mut().insert(id, viewport.clone()));

    let w = w.disabled(move || disabled.get());

    let w = w.on_event(floem::event::EventListener::WindowResized, move |s| {
        if let Event::WindowResized(s) = s {
            viewport.set(Rect::new(0., 0., s.width, s.height));
        }
        floem::EventPropagation::Continue
    });

    w
}
