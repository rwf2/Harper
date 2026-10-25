use super::Url;

#[test]
fn relative_part() {
    let cases = [
        ("", ""),
        ("foo/bar", "foo/bar"),
        ("/foo/bar", "foo/bar"),
        ("https://rocket.rs/foo?x=1#top", "foo?x=1#top"),
        ("//rocket.rs/foo", "foo"),
        ("https://rocket.rs", ""),
        ("//rocket.rs/", ""),
        ("https://rocket.rs?next=/foo#top", "?next=/foo#top"),
        ("//rocket.rs#foo/bar", "#foo/bar"),
        ("https://user:pass@[::1]:443/foo", "foo"),
        ("foo?url=https://rocket.rs/", "foo?url=https://rocket.rs/"),
        ("#https://rocket.rs/foo", "#https://rocket.rs/foo"),
        ("ftp:/foo", "foo"),
        ("mailto:foo@bar.com", "foo@bar.com"),
    ];

    for (input, expected) in cases {
        assert_eq!(Url::new(input).relative_part().as_str(), expected, "{input}");
    }
}
