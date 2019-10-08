use super::*;

#[derive(Clone, new)]
pub struct Border<T: Ui, L: Value<LayoutSideOffsets>, B: Value<BorderDetails>> {
    child: T,
    widths: L,
    details: B,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

#[impl_ui_crate]
impl<T: Ui, L: Value<LayoutSideOffsets>, B: Value<BorderDetails>> Ui for Border<T, L, B> {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut result = self.child.measure(available_size);
        result.width += self.widths.left + self.widths.right;
        result.height += self.widths.top + self.widths.bottom;

        result
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width -= self.widths.left + self.widths.right;
        final_size.height -= self.widths.top + self.widths.bottom;

        self.child.arrange(final_size)
    }

    fn value_changed(&mut self, _: &mut UiValues, update: &mut NextUpdate) {
        if self.widths.changed() {
            update.update_layout();
        }

        if self.details.changed() {
            update.render_frame();
        }
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag).or_else(|| {
            self.child.point_over(hits).map(|mut lp| {
                lp.x += self.widths.left;
                lp.y += self.widths.top;
                lp
            })
        })
    }

    fn render(&self, f: &mut NextFrame) {
        let offset = LayoutPoint::new(self.widths.left, self.widths.top);
        let mut size = f.final_size();
        size.width -= self.widths.left + self.widths.right;
        size.height -= self.widths.top + self.widths.bottom;

        f.push_child(&self.child, &LayoutRect::new(offset, size));

        f.push_border(
            LayoutRect::from_size(f.final_size()),
            *self.widths,
            *self.details,
            Some(self.hit_tag),
        );
    }
}

pub trait BorderExtt: Ui + Sized {
    fn border<L: IntoValue<LayoutSideOffsets>, B: IntoValue<BorderDetails>>(
        self,
        widths: L,
        details: B,
    ) -> Border<Self, L::Value, B::Value> {
        Border::new(self, widths.into_value(), details.into_value())
    }
}
impl<T: Ui> BorderExtt for T {}
