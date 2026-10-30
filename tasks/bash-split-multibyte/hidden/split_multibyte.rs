const MULTIBYTE: &[&str] = &["你好", "café", "🦀", "naïve 世界 🎉"];
const SEPARATORS: &[&str] = &["|", ";", "&&", "||"];
/// `{t}` is replaced by the text, `{sep}` by the separator.
const TEMPLATES: &[&str] = &[
    "echo {t} {sep} ls {t}",
    "echo {t}{sep}ls",
    "echo {t} {sep} rm -rf /tmp/{t}",
    "cat {t}.txt {sep} git push --force",
    "echo '{t} {sep} rm -rf /' {sep} ls {t}",
    "echo \"{t} {sep} rm -rf /\" {sep} grep {t}",
    "grep \"{t}\" a.txt {sep} echo '{t}'",
    "FOO='{t}' ls {sep} echo {t}",
    "FOO=\"{t} x\" ls {sep} rm -rf {t}",
    "{t} {sep} {t}",
];

#[test]
fn multibyte_commands_split_like_their_ascii_equivalents() {
    for tpl in TEMPLATES {
        for sep in SEPARATORS {
            let fill = |t: &str| tpl.replace("{t}", t).replace("{sep}", sep);
            for t in MULTIBYTE {
                let (cmd, ascii) = (fill(t), fill("X"));
                let expected: Vec<String> = split_command_segments(&ascii)
                    .iter()
                    .map(|seg| seg.replace('X', t))
                    .collect();
                assert_eq!(split_command_segments(&cmd), expected, "segments of {cmd:?}");
            }
        }
    }
}

#[test]
fn multibyte_commands_split_into_the_exact_segments() {
    assert_eq!(
        split_command_segments("echo 你好 | grep 好"),
        vec!["echo 你好 ", " grep 好"]
    );
    assert_eq!(
        split_command_segments("echo café&&ls🦀||echo 'é;é';true"),
        vec!["echo café", "ls🦀", "echo 'é;é'", "true"]
    );
    assert_eq!(
        split_command_segments("echo \"好 | 好\""),
        vec!["echo \"好 | 好\""]
    );
}
