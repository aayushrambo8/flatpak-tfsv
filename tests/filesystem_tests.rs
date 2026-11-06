use std::path::Path;

use tfsv::filesystem::{ascii_safe_display, validate_ascii_output};

#[test]
fn safe_display_preserves_unicode_and_escapes_controls() {
    let path = Path::new("report-✓.txt");
    let safe = ascii_safe_display(path);
    assert_eq!(safe, "report-✓.txt");

    let controlled = ascii_safe_display(Path::new("bad\nname.txt"));
    assert_eq!(controlled, "bad\\u{A}name.txt");
}

#[test]
fn ascii_outputs_are_only_visible_ascii() {
    let text = "safe-name_123";
    assert!(validate_ascii_output(text));
}
