use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn Error>>;
const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12" viewBox="0 0 24 12"><path d="M0 0H24V12H0Z" fill="#123456"/></svg>"##;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_svg2vector"))
}

fn svg_file(directory: &Path, name: &str) -> Result<std::path::PathBuf, std::io::Error> {
    let path = directory.join(name);
    fs::write(&path, SVG)?;
    Ok(path)
}

fn assert_vector(xml: &str) -> TestResult {
    let document = roxmltree::Document::parse(xml)?;
    assert_eq!(document.root_element().tag_name().name(), "vector");
    assert!(document.descendants().any(|node| node.has_tag_name("path")));
    Ok(())
}

#[test]
fn shows_usage_when_help_is_requested() -> TestResult {
    // Given / When
    let output = command().arg("--help").output()?;
    // Then
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("--canvas"));
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn prints_vector_when_single_file_has_no_output_option() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = svg_file(directory.path(), "icon.svg")?;
    // When
    let output = command().arg(input).output()?;
    // Then
    assert!(output.status.success(), "{:?}", output);
    assert_vector(&String::from_utf8(output.stdout)?)?;
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn prints_vector_when_input_is_stdin() -> TestResult {
    // Given
    let mut child = command()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("missing stdin pipe")?;
    stdin.write_all(SVG.as_bytes())?;
    drop(stdin);
    // When
    let output = child.wait_with_output()?;
    // Then
    assert!(output.status.success());
    assert_vector(&String::from_utf8(output.stdout)?)
}

#[test]
fn preserves_existing_output_when_force_is_absent() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = svg_file(directory.path(), "icon.svg")?;
    let target = directory.path().join("icon.xml");
    fs::write(&target, "existing content")?;
    // When
    let output = command().arg(input).arg("-o").arg(&target).output()?;
    // Then
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(target)?, "existing content");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    Ok(())
}

#[test]
fn replaces_existing_output_when_force_is_set() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = svg_file(directory.path(), "icon.svg")?;
    let target = directory.path().join("icon.xml");
    fs::write(&target, "existing content")?;
    // When
    let output = command()
        .arg(input)
        .arg("-o")
        .arg(&target)
        .arg("--force")
        .output()?;
    // Then
    assert!(output.status.success(), "{:?}", output);
    assert_vector(&fs::read_to_string(target)?)?;
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn rejects_invalid_svg_without_replacing_output() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("broken.svg");
    let target = directory.path().join("icon.xml");
    fs::write(&input, "<svg>")?;
    fs::write(&target, "existing content")?;
    // When
    let output = command()
        .arg(input)
        .arg("-o")
        .arg(&target)
        .arg("--force")
        .output()?;
    // Then
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(target)?, "existing content");
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn converts_direct_svg_children_when_input_is_directory() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("input");
    let target = directory.path().join("output");
    fs::create_dir(&input)?;
    svg_file(&input, "b.svg")?;
    svg_file(&input, "a.svg")?;
    fs::write(input.join("ignored.txt"), "not SVG")?;
    fs::create_dir(input.join("nested"))?;
    fs::write(input.join("nested/broken.svg"), "<svg>")?;
    // When
    let output = command().arg(input).arg("-o").arg(&target).output()?;
    // Then
    assert!(output.status.success(), "{:?}", output);
    assert_vector(&fs::read_to_string(target.join("a.xml"))?)?;
    assert_vector(&fs::read_to_string(target.join("b.xml"))?)?;
    assert_eq!(fs::read_dir(target)?.count(), 2);
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn leaves_batch_unwritten_when_any_source_is_malformed() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("input");
    let target = directory.path().join("output");
    fs::create_dir(&input)?;
    svg_file(&input, "a.svg")?;
    fs::write(input.join("z.svg"), "<svg>")?;
    // When
    let output = command().arg(input).arg("-o").arg(&target).output()?;
    // Then
    assert!(!output.status.success());
    assert!(!target.exists());
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn rejects_directory_without_output_option() -> TestResult {
    // Given
    let directory = tempdir()?;
    svg_file(directory.path(), "icon.svg")?;
    // When
    let output = command().arg(directory.path()).output()?;
    // Then
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn rejects_invalid_android_output_names() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = svg_file(directory.path(), "icon.svg")?;
    let target = directory.path().join("Bad-Name.xml");
    // When
    let output = command().arg(input).arg("-o").arg(&target).output()?;
    // Then
    assert!(!output.status.success());
    assert!(!target.exists());
    Ok(())
}

#[test]
fn rejects_invalid_dimensions_before_reading_stdin() -> TestResult {
    // Given / When
    let output = command().args(["-", "--canvas", "24xNaN"]).output()?;
    // Then
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    Ok(())
}

#[test]
fn applies_dimensions_and_canvas_when_options_are_set() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = svg_file(directory.path(), "icon.svg")?;
    // When
    let output = command()
        .arg(input)
        .args(["--width", "40", "--height", "30", "--canvas", "48x36"])
        .output()?;
    // Then
    assert!(output.status.success(), "{:?}", output);
    let xml = String::from_utf8(output.stdout)?;
    let document = roxmltree::Document::parse(&xml)?;
    let root = document.root_element();
    let android = "http://schemas.android.com/apk/res/android";
    assert_eq!(root.attribute((android, "width")), Some("40dp"));
    assert_eq!(root.attribute((android, "height")), Some("30dp"));
    assert_eq!(root.attribute((android, "viewportWidth")), Some("48"));
    assert_eq!(root.attribute((android, "viewportHeight")), Some("36"));
    Ok(())
}

#[test]
fn rejects_dotted_names_without_colliding_batch_outputs() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("input");
    let target = directory.path().join("output");
    fs::create_dir(&input)?;
    svg_file(&input, "icon.svg")?;
    svg_file(&input, "icon.dark.svg")?;
    // When
    let output = command().arg(input).arg("-o").arg(&target).output()?;
    // Then
    assert!(!output.status.success());
    assert!(!target.exists());
    Ok(())
}

#[test]
fn leaves_batch_unwritten_when_later_output_exists() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("input");
    let target = directory.path().join("output");
    fs::create_dir(&input)?;
    fs::create_dir(&target)?;
    svg_file(&input, "a.svg")?;
    svg_file(&input, "z.svg")?;
    fs::write(target.join("z.xml"), "existing")?;
    // When
    let output = command().arg(input).arg("-o").arg(&target).output()?;
    // Then
    assert!(!output.status.success());
    assert!(!target.join("a.xml").exists());
    assert_eq!(fs::read_to_string(target.join("z.xml"))?, "existing");
    Ok(())
}

#[test]
fn reports_first_sorted_source_when_multiple_files_are_invalid() -> TestResult {
    // Given
    let directory = tempdir()?;
    fs::write(directory.path().join("z.svg"), "<svg>")?;
    fs::write(directory.path().join("a.svg"), "<svg>")?;
    // When
    let output = command()
        .arg(directory.path())
        .arg("-o")
        .arg(directory.path().join("out"))
        .output()?;
    // Then
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr)?;
    assert!(error.contains("a.svg"));
    assert!(!error.contains("z.svg"));
    Ok(())
}

#[test]
fn preserves_input_when_output_names_the_same_file() -> TestResult {
    // Given
    let directory = tempdir()?;
    let input = svg_file(directory.path(), "icon.xml")?;
    // When
    let output = command()
        .arg(&input)
        .arg("-o")
        .arg(&input)
        .arg("--force")
        .output()?;
    // Then
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(input)?, SVG);
    Ok(())
}
