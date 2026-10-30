#[test]
fn newline_does_not_hide_a_second_command() {
    for cmd in ["ls\nrm -rf /", "ls\r\nrm -rf /", "ls\n\n  rm -rf /\n"] {
        assert_ne!(classify_bash_command(cmd), ToolEffect::ReadOnly, "{cmd:?}");
    }
}

#[test]
fn background_ampersand_does_not_hide_a_second_command() {
    for cmd in ["ls & rm -rf /", "ls&rm -rf /", "sleep 1 & rm -rf / &"] {
        assert_ne!(classify_bash_command(cmd), ToolEffect::ReadOnly, "{cmd:?}");
    }
}

#[test]
fn a_redirect_does_not_hide_a_destructive_second_command() {
    // Not-read-only is not enough here: this one was already a local
    // mutation, which Auto mode approves. `rm -rf /` alone is Destructive.
    assert_eq!(
        classify_bash_command("rm -rf /"),
        ToolEffect::Destructive,
        "guard"
    );
    for cmd in ["echo hi > f; rm -rf /", "echo hi > f && rm -rf /"] {
        assert_eq!(
            classify_bash_command(cmd),
            ToolEffect::Destructive,
            "{cmd:?}"
        );
    }
}

#[test]
fn an_escaped_quote_does_not_hide_a_substitution() {
    for cmd in ["echo it\\'s; echo $(rm -rf /)", "echo it\\'s; rm -rf /"] {
        assert_ne!(classify_bash_command(cmd), ToolEffect::ReadOnly, "{cmd:?}");
    }
}

#[test]
fn harmless_redirects_stay_read_only() {
    for cmd in [
        "git status 2>&1",
        "find . 2>/dev/null",
        "ls >/dev/null",
        "git log --oneline && git status",
        "grep 'a | b' file",
    ] {
        assert_eq!(classify_bash_command(cmd), ToolEffect::ReadOnly, "{cmd:?}");
    }
}
