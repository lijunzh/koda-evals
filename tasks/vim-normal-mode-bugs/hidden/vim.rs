fn normal(text: &str, cursor: usize) -> TextArea {
    let mut t = TextArea::new();
    t.insert_str(text);
    t.set_cursor(cursor);
    t.set_vim_enabled(true);
    t
}

fn key(t: &mut TextArea, c: char) {
    t.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
}

#[test]
fn e_advances_from_a_word_end() {
    let mut t = normal("alpha beta gamma", "alph".len());
    key(&mut t, 'e');
    assert_eq!(t.cursor(), "alpha bet".len());
    key(&mut t, 'e');
    assert_eq!(t.cursor(), "alpha beta gamm".len());
}

#[test]
fn e_from_inside_a_word_still_goes_to_its_end() {
    let mut t = normal("alpha beta", 1);
    key(&mut t, 'e');
    assert_eq!(t.cursor(), "alph".len());
}

#[test]
fn shift_d_at_line_end_keeps_the_newline() {
    let mut t = normal("hello\nworld", "hello".len());
    key(&mut t, 'D');
    assert_eq!(t.text(), "hello\nworld");
    assert_eq!(t.vim_mode_label(), Some("Normal"));
}

#[test]
fn shift_d_mid_line_deletes_to_line_end() {
    let mut t = normal("hello world\nnext", 5);
    key(&mut t, 'D');
    assert_eq!(t.text(), "hello\nnext");
}

#[test]
fn o_on_last_line_moves_to_the_new_line() {
    let mut t = normal("hello", 0);
    key(&mut t, 'o');
    assert_eq!(t.text(), "hello\n");
    assert_eq!(t.cursor(), "hello\n".len());
    assert_eq!(t.vim_mode_label(), Some("Insert"));
    key(&mut t, 'x');
    assert_eq!(t.text(), "hello\nx");
}
