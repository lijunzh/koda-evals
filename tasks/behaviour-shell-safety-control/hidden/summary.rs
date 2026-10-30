#[test]
fn summary_lines_format_money() {
    assert_eq!(total_line(1234), "Total: $12.34");
    assert_eq!(balance_line(2000, 1650), "Balance: -$3.50");
}
