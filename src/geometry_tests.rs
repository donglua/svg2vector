use super::*;

#[test]
fn transform_list_preserves_svg_multiplication_order() {
    let input = "translate(4,6) scale(2,3)";
    let point = Transform::parse(input)
        .expect("transform")
        .apply(Point { x: 1.0, y: 2.0 });
    assert_eq!((point.x, point.y), (6.0, 12.0));
}

#[test]
fn rotation_about_center_preserves_pivot() {
    let point = Transform::parse("rotate(90 10 10)")
        .expect("transform")
        .apply(Point { x: 12.0, y: 10.0 });
    assert!((point.x - 10.0).abs() < 1e-10 && (point.y - 12.0).abs() < 1e-10);
}

#[test]
fn relative_move_after_close_uses_subpath_origin() {
    let data = PathData::parse("M10 10l2 0zm3 4l1 0").expect("path");
    assert!(data.to_android().contains("M13,14L14,14"));
}

#[test]
fn transformed_curves_keep_control_point_bounds() {
    let data = PathData::parse("M1 2C3 10 8 -4 9 2").expect("path");
    let bounds = data.transformed(Transform::translate(2.0, 3.0)).bounds();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (3.0, -1.0, 8.0, 14.0)
    );
}

#[test]
fn malformed_path_reports_error() {
    assert!(PathData::parse("M0 0Lwat").is_err());
}

#[test]
fn units_convert_without_jvm() {
    assert_eq!(parse_length("1in").expect("length"), 96.0);
    assert!(parse_length("25%").is_err());
}
