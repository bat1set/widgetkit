use crate::{
    Color, Constraints, Insets, InstanceId, Rect, Size, SizePolicy, TaskId, TimerId, WidgetId,
};

#[test]
fn rect_inset_clamps_to_zero() {
    let rect = Rect::xywh(0.0, 0.0, 10.0, 5.0);
    let inset = rect.inset(Insets::all(10.0));
    assert_eq!(inset, Rect::xywh(10.0, 10.0, 0.0, 0.0));
}

#[test]
fn color_alpha_override_keeps_rgb() {
    let color = Color::rgb(10, 20, 30).with_alpha(40);
    assert_eq!(color, Color::rgba(10, 20, 30, 40));
}

#[test]
fn ids_are_unique() {
    assert_ne!(WidgetId::new(), WidgetId::new());
    assert_ne!(InstanceId::new(), InstanceId::new());
    assert_ne!(TimerId::new(), TimerId::new());
    assert_ne!(TaskId::new(), TaskId::new());
}

#[test]
fn constraints_clamp_size_between_limits() {
    let constraints = Constraints::new(Some(Size::new(40.0, 20.0)), Some(Size::new(80.0, 60.0)));

    assert_eq!(
        constraints.clamp(Size::new(10.0, 100.0)),
        Size::new(40.0, 60.0)
    );
}

#[test]
fn fixed_size_policy_maps_to_fixed_constraints() {
    let size = Size::new(120.0, 48.0);

    assert_eq!(
        SizePolicy::Fixed(size).constraints(),
        Constraints::new(Some(size), Some(size))
    );
}
