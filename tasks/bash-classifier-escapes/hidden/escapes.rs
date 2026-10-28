#[test]
fn env_running_a_command_is_not_read_only() {
    for cmd in [
        "env cargo build",
        "env make install",
        "env FOO=bar rm file",
        "env PATH=/tmp ls /",
        "env -i bash",
    ] {
        assert_ne!(classify_bash_command(cmd), ToolEffect::ReadOnly, "{cmd}");
    }
}

#[test]
fn find_actions_that_change_things_are_destructive() {
    for cmd in [
        "find . -name '*.tmp' -delete",
        "find /tmp -delete",
        "find . -name '*.tmp' -exec rm {} ;",
        "find . -execdir rm {} +",
        "find /var/log -exec gzip {} ;",
        "find . -fprint /tmp/out",
        "find / -fprintf /tmp/out '%p\\n'",
        "find . -fls /tmp/out",
        "find . -name '*.tmp' -ok rm {} ;",
        "find . -okdir mv {} /tmp ;",
    ] {
        assert_eq!(classify_bash_command(cmd), ToolEffect::Destructive, "{cmd}");
    }
}

#[test]
fn read_only_searches_stay_read_only() {
    for cmd in [
        "find .",
        "find . -name '*.rs'",
        "find . -type f -size +1M",
        "find . -newer reference.txt",
        "find . -mtime -7",
        "printenv",
        "printenv PATH",
        "ls -la",
        "git status",
    ] {
        assert_eq!(classify_bash_command(cmd), ToolEffect::ReadOnly, "{cmd}");
    }
}
