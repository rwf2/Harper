fn test_char_table(table: &[u8]) {
    for (i, &v) in table.iter().enumerate() {
        if v != 0 {
            assert_eq!(i, v as usize);
        }
    }
}

#[test]
fn check_tables() {
    test_char_table(&super::URL_CHARS[..]);
}
