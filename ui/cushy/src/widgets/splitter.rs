use cushy::context::{AsEventContext, EventContext, Trackable};
use cushy::figures::units::{Lp, Px, UPx};
use cushy::figures::{ScreenScale, Size};
use cushy::value::{Dynamic, IntoValue, Source, Value};
use cushy::widget::{MakeWidget, MakeWidgetList, MountedChildren, Widget, WidgetList, WidgetRef};
use cushy::widgets::grid::Orientation;
use cushy::ConstraintLimit;

#[derive(Debug)]
pub struct Splitter {
    orientation: Orientation,
    width: Dynamic<UPx>,
    children: Value<WidgetList>,
    mounted: MountedChildren,
}

impl Splitter {
    pub fn new(left: impl MakeWidget, right: impl MakeWidget, width: Dynamic<UPx>) -> Self {
        let mut children = WidgetList::new();
        children.push(left);
        children.push(right);
        Splitter {
            orientation: Orientation::Row,
            width,
            children: children.into_value(),
            mounted: MountedChildren::default(),
        }
    }
    fn synchronize_children(&mut self, context: &mut EventContext<'_>) {
        self.children.invalidate_when_changed(context);
        self.mounted.synchronize_with(&self.children, context);
    }
}

impl Widget for Splitter {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        for children in self.mounted.children() {
            context.for_other(children).redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<UPx> {
        self.synchronize_children(&mut context.as_event_context());
        let mut width = self.width.get().into_upx(context.gfx.scale());
        let mut height = available_space.height;
        let mut x = 0.0;
        let mut y = 0.0;

        let left = self.mounted.children().first().unwrap();
        let right = self.mounted.children().get(1).unwrap();
        let size = if self.orientation == Orientation::Row {
            let l = context.for_other(left).layout(Size::new(ConstraintLimit::Fill(width), available_space.height));
            let r =   context.for_other(right).layout(Size::new(available_space.width - width, available_space.height));
            l + r
        } else {
            let l = context.for_other(left).layout(Size::new(available_space.width,ConstraintLimit::Fill(width)));
            let r = context.for_other(right).layout(Size::new(available_space.height, available_space.width - width));
            l + r
        };

        size
    }
}
