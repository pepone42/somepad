use floem::{
    action::open_file,
    event::Event,
    file::FileDialogOptions,
    id::Id,
    keyboard::{Key, ModifiersState, NamedKey},
    reactive::{create_rw_signal, RwSignal},
    view::{View, ViewData, Widget},
    views::{container, Container, Decorators},
    EventPropagation,
};
use ndoc::Document;

use crate::documents::Documents;

use super::palette;

pub fn window<V: View + 'static>(child: V, documents: RwSignal<Documents>) -> Container {
    let w = container(child);

    let w = w.on_key_down(
        Key::Character("n".into()),
        ModifiersState::CONTROL,
        move |_| {
            let doc = create_rw_signal(Document::default());
            documents.update(|d| d.add(doc));
        },
    );

    let w = w.on_key_down(
        Key::Character("o".into()),
        ModifiersState::CONTROL,
        move |_| {
            let doc = create_rw_signal(Document::default());
            open_file(FileDialogOptions::new().title("Open new file"), move |p| {
                if let Some(path) = p {
                    doc.set(Document::from_file(&path.path[0]).unwrap());
                    documents.update(|d| d.add(doc));
                }
            });
        },
    );

    let w = w.on_key_down(
        Key::Character("p".into()),
        ModifiersState::CONTROL,
        move |_| {
            if !documents.get().is_empty() {
                palette(
                    documents
                        .get()
                        .order_by_mru()
                        .iter()
                        .enumerate()
                        .map(|(_, d)| (d.get().id(), d.get().title().to_string()))
                        .collect(),
                    move |i| documents.update(|d| d.set_current(i)),
                );
            }
        },
    );

    w
}
