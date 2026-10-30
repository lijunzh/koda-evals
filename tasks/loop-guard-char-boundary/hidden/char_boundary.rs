// Through `record`, not the private fingerprint: any cut that keeps calls
// apart passes, whether it lands before or after the character.

fn call(args: &str) -> ToolCall {
    ToolCall {
        id: "x".into(),
        function_name: "Bash".into(),
        arguments: args.into(),
        thought_signature: None,
    }
}

/// '─' is 3 bytes: 198 and 199 put byte 200 inside it, 200 on its edge.
const PADDING: [usize; 3] = [198, 199, 200];

#[test]
fn a_multibyte_char_across_the_cut_does_not_panic() {
    for pad in PADDING {
        let args = format!("{}{}", "a".repeat(pad), "─".repeat(10));
        let mut d = LoopDetector::new();
        assert_eq!(d.record(&[call(&args)]), LoopAction::Ok, "padding {pad}");
    }
}

#[test]
fn repeats_with_multibyte_args_are_still_detected() {
    for pad in PADDING {
        let args = format!("{}{}", "a".repeat(pad), "─".repeat(10));
        let mut d = LoopDetector::new();
        let actions: Vec<LoopAction> = (0..5).map(|_| d.record(&[call(&args)])).collect();
        assert!(
            matches!(actions.last(), Some(LoopAction::InjectFeedback(_))),
            "padding {pad}: {actions:?}"
        );
    }
}

#[test]
fn different_commands_with_multibyte_args_stay_apart() {
    for pad in PADDING {
        let mut d = LoopDetector::new();
        for i in 0..5 {
            let args = format!("{i}{}{}", "a".repeat(pad - 1), "─".repeat(10));
            assert_eq!(d.record(&[call(&args)]), LoopAction::Ok, "padding {pad}");
        }
    }
}
