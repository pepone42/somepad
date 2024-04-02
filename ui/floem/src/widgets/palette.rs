use floem::{
    action::{add_overlay, remove_overlay},
    kurbo::Point,
    peniko::Color,
    views::{label, virtual_list, Decorators, VirtualDirection, VirtualItemSize},
};

pub fn palette(
    //items: im::Vector<(usize, String)>,
    items: impl Iterator<Item = (usize, String)>,
    action: impl FnOnce(usize) + 'static + Clone + Copy,
) {
    let items = im::Vector::from_iter(items);
    add_overlay(Point::new(0., 0.), move |id| {
        virtual_list(
            VirtualDirection::Vertical,
            VirtualItemSize::Fixed(Box::new(|| 20.0)),
            move || items.clone(),
            move |item| item.clone(),
            move |item| {
                label(move || item.1.to_string())
                    .style(|s| s.height(20.0))
                    .on_click_stop(move |_| {
                        action(item.0);
                        remove_overlay(id);
                    })
            },
        )
        .style(|s| s.background(Color::WHITE))
    });
}
