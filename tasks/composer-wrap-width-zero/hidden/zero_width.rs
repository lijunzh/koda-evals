fn area_text(t: &TextArea, width: u16, height: u16) {
    let area = Rect::new(0, 0, width, height);
    let _ = t.cursor_pos_with_state(area, TextAreaState::default());
    let mut buf = Buffer::empty(area);
    Widget::render(t, area, &mut buf);
}

#[test]
fn zero_width_with_newlines_does_not_panic() {
    for text in ["\n", "a\n", "a\n\nb", "\n\n", "hello world\n"] {
        let mut t = TextArea::new();
        t.insert_str(text);
        let lines = text.split('\n').count() as u16;
        assert!(t.desired_height(0) >= lines, "{text:?}");
        area_text(&t, 0, 0);
        area_text(&t, 0, 5);
    }
}

#[test]
fn normal_widths_unchanged() {
    let mut t = TextArea::new();
    t.insert_str("a\n\nb");
    assert_eq!(t.desired_height(80), 3);
    area_text(&t, 80, 3);
}
