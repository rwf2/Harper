#[test]
fn test_slugify() {
    use crate::util::slugify;

    assert_eq!(slugify("My Test String!!!1!1"), "my-test-string-1-1");
    assert_eq!(slugify("test\nit   now!"), "test-it-now");
    assert_eq!(slugify("\tfoo\r\nbar\u{a0}baz\u{2003}"), "foo-bar-baz");
    assert_eq!(slugify("  --test_-_cool- -  "), "test_-_cool");
    assert_eq!(slugify("Æúű--cool?"), "aeuu-cool");
    assert_eq!(slugify("You & Me"), "you-me");
    assert_eq!(slugify("  user@-- example.com  "), "user-example-com");
}
