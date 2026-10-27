/// Any correct cut: a prefix of `text`, at most `max` bytes, as long as it
/// can be (within one character of `max`), plus `…` only when cut.
fn check(text: &str, max: usize) {
    let out = preview(text, max);
    if text.len() <= max {
        assert_eq!(out, text, "text that fits must be unchanged");
        return;
    }
    let body = out
        .strip_suffix('…')
        .unwrap_or_else(|| panic!("a cut preview must end with …: {out:?}"));
    assert!(text.starts_with(body), "{body:?} is not a prefix of {text:?}");
    assert!(body.len() <= max, "{body:?} is longer than {max} bytes");
    assert!(max - body.len() < 4, "{body:?} cut more than one character short of {max} bytes");
}

#[test]
fn multibyte_text_never_panics() {
    for text in ["héllo wörld", "🐻🐻🐻 bear", "思考中です", "a🤔b"] {
        for max in 0..=text.len() + 1 {
            check(text, max);
        }
    }
}

#[test]
fn ascii_behavior_is_kept() {
    assert_eq!(preview("hello world", 5), "hello…");
    assert_eq!(preview("hello", 5), "hello");
    assert_eq!(preview("hello", 0), "…");
}
