#[test]
fn keeps_one_x_leniency_and_key_case() {
    let text = "# server\nHost = 0.0.0.0\nstray words\nPort = 9000\nWORKERS = 8\n";
    assert_eq!(
        load(text),
        Ok(Settings {
            host: "0.0.0.0".into(),
            port: 9000,
            workers: 8
        })
    );
    assert_eq!(banner(text), "listening on 0.0.0.0:9000 (8 workers)");
}

#[test]
fn defaults_and_errors() {
    assert_eq!(banner(""), "listening on 127.0.0.1:8080 (4 workers)");
    assert!(banner("port = 70000").starts_with("bad config: port must be"));
    assert!(banner("workers = many").starts_with("bad config: workers must be"));
    assert!(banner("= x").starts_with("bad config: line 1"));
}
