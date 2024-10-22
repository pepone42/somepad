use cushy::{
    context::{AsEventContext, LayoutContext},
    figures::{units::Px, IntoUnsigned, Size},
    value::{Dynamic, Source},
    widget::{MakeWidget, WidgetRef, WrappedLayout, WrapperWidget},
    ConstraintLimit,
};

#[derive(Debug)]
pub struct SideBar {
    child: WidgetRef,
    width: Dynamic<Px>,
}

impl SideBar {
    pub fn new(child: impl MakeWidget, width: Dynamic<Px>) -> Self {
        Self {
            child: child.make_widget().widget_ref(),
            width,
        }
    }
}

impl WrapperWidget for SideBar {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        context.invalidate_when_changed(&self.width);
        let child = self.child.mounted(&mut context.as_event_context());

        let available_space = Size::new(
            ConstraintLimit::Fill(self.width.get().into_unsigned()),
            available_space.height,
        );

        let size = context.for_other(&child).layout(available_space);

        Size::new(self.width.get().into_unsigned(), size.height).into()
    }
}
