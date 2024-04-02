use floem::{
    action::open_file,
    file::FileDialogOptions,
    reactive::{create_rw_signal, RwSignal},
    view::View,
    views::{container, Container},
};
use ndoc::Document;

use crate::{documents::Documents, shortcut};
use crate::decorators::CustomDecorators;

use super::palette;

pub fn window<V: View + 'static>(child: V, documents: RwSignal<Documents>) -> Container {
    let w = container(child);

    let w = w.on_shortcut(shortcut!(Ctrl+n),
        move |_| {
            let doc = create_rw_signal(Document::default());
            documents.update(|d| d.add(doc));
        },
    );

    let w = w.on_shortcut(shortcut!(Ctrl+o),
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

    let w = w.on_shortcut(shortcut!(Ctrl+p),
        move |_| {
            if !documents.get().is_empty() {
                palette(
                    documents
                        .get()
                        .order_by_mru()
                        .iter()
                        .enumerate()
                        .map(|(_, d)| (d.get().id(), d.get().title().to_string())),
                    move |i| documents.update(|d| d.set_current(i)),
                );
            }
        },
    );

    w
}
