use floem::{
    action::open_file,
    event::Event,
    file::FileDialogOptions,
    kurbo::{Rect, Size},
    reactive::{create_effect, create_rw_signal, RwSignal},
    view::{self, View},
    views::{container, Container, Decorators},
};
use ndoc::Document;

use crate::decorators::CustomDecorators;
use crate::{documents::Documents, shortcut};

use super::palette;

pub fn window<V: View + 'static>(child: V, documents: RwSignal<Documents>) -> Container {
    let w = container(child);

    let id = w.id();
    let disabled = create_rw_signal(false);
    let size = create_rw_signal(Size::new(100., 100.));
    let palette_viewport = create_rw_signal(Rect::new(0., 0., 100., 100.));

    // create_effect(move |_| {
    //     if disabled.get() {
    //         id.clear_focus();
    //     } else {
    //         id.request_focus();
    //     }
    // });

    create_effect(move |_| {
        let s = size.get();
        palette_viewport.set(Rect::new(0., 0., s.width, s.height));
    });

    let w = w.disabled(move || disabled.get());

    let w = w.on_shortcut(shortcut!(Ctrl + n), move |_| {
        let doc = create_rw_signal(Document::default());
        documents.update(|d| d.add(doc));
    });

    let w = w.on_shortcut(shortcut!(Ctrl + o), move |_| {
        let doc = create_rw_signal(Document::default());
        open_file(FileDialogOptions::new().title("Open new file"), move |p| {
            if let Some(path) = p {
                doc.set(Document::from_file(&path.path[0]).unwrap());
                documents.update(|d| d.add(doc));
            }
        });
    });

    let w = w.on_event(floem::event::EventListener::WindowResized, move |s| {
        if let Event::WindowResized(s) = s {
            size.set(*s);
        }
        floem::EventPropagation::Continue
    });

    let w = w.on_shortcut(shortcut!(Ctrl + p), move |_| {
        disabled.set(true);
        if !documents.get().is_empty() {
            palette(
                palette_viewport.get(),
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
        documents.update(|d| d.remove(d.current_id()));
    });

    w
}
