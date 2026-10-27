// The regression tests #32 added (koda commit 77b857f).

#[test]
fn multibyte_emoji_no_panic() {
    // Buffer " 🐻** is" is 10 bytes; keeping the last 7 for a possible
    // "<think>" leaves byte 3, inside 🐻.
    let mut filter = ThinkTagFilter::new();
    let mut all = Vec::new();
    all.extend(filter.process(StreamChunk::TextDelta(" 🐻** is".into())));
    all.extend(filter.flush());

    let text: String = all
        .iter()
        .filter_map(|c| match c {
            StreamChunk::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, " 🐻** is");
}

#[test]
fn multibyte_in_think_block() {
    let mut filter = ThinkTagFilter::new();
    let mut all = Vec::new();
    all.extend(filter.process(StreamChunk::TextDelta("<think>思考中🤔</think>答え".into())));
    all.extend(filter.flush());

    let thinking: String = all
        .iter()
        .filter_map(|c| match c {
            StreamChunk::ThinkingDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    let text: String = all
        .iter()
        .filter_map(|c| match c {
            StreamChunk::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(thinking, "思考中🤔");
    assert_eq!(text, "答え");
}
