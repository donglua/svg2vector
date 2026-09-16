use svg2vector::{Options, convert};

#[test]
fn rectangle_converts_to_android_vector() {
    let svg = r##"<svg width="20" height="20" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg"><rect x="2" y="3" width="10" height="8" fill="#123456"/></svg>"##;
    let xml = convert(svg, &Options::default()).expect("convert a basic rectangle");
    let parsed = roxmltree::Document::parse(&xml).expect("well formed XML");
    assert_eq!(parsed.root_element().tag_name().name(), "vector");
    let path = parsed
        .descendants()
        .find(|n| n.has_tag_name("path"))
        .expect("drawable path");
    assert_eq!(
        path.attribute(("http://schemas.android.com/apk/res/android", "fillColor")),
        Some("#123456")
    );
}

#[test]
fn dimensions_that_would_round_to_zero_are_rejected() {
    let tiny =
        r#"<svg viewBox="0 0 0.0000001 0.0000001"><path d="M0 0L0.0000001 0.0000001"/></svg>"#;
    assert!(
        convert(tiny, &Options::default())
            .unwrap_err()
            .to_string()
            .contains("too small")
    );
    let svg = r#"<svg viewBox="0 0 24 24"><rect width="24" height="24"/></svg>"#;
    let options = Options {
        width: Some(1e-7),
        ..Options::default()
    };
    assert!(
        convert(svg, &options)
            .unwrap_err()
            .to_string()
            .contains("too small")
    );
}
