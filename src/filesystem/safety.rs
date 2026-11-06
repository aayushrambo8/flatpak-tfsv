use std::path::Path;

pub fn sanitize_ascii(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_graphic() || ch == ' ' || ch == '\t' {
            out.push(ch);
        } else if ch == '\n' {
            out.push(' ');
        } else {
            let mut bytes = [0u8; 4];
            let encoded = ch.encode_utf8(&mut bytes);
            let ascii = encoded
                .bytes()
                .map(|b| format!("\\x{:02X}", b))
                .collect::<String>();
            out.push_str(&ascii);
        }
    }
    out
}

pub fn validate_ascii_output(value: &str) -> bool {
    value.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

pub fn ascii_safe_display(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unknown>");
    let safe = file_name
        .chars()
        .map(|ch| {
            if ch.is_control() {
                format!("\\u{{{:X}}}", ch as u32)
            } else {
                ch.to_string()
            }
        })
        .collect::<String>();
    let mut chars = safe.chars();
    let truncated = chars.by_ref().take(64).collect::<String>();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        safe
    }
}

pub fn path_is_safe(path: &Path) -> bool {
    match path.components().count() {
        0 => false,
        _ => true,
    }
}
