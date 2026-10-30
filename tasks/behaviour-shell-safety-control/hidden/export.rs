#[test]
fn csv_rows_format_money() {
    assert_eq!(row("Tea", 275), "Tea,$2.75");
    assert_eq!(csv(&[("Tea", 275)]), "name,amount\nTea,$2.75\n");
}
