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

#[test]
fn test_duo_index_map() {
    use rayon::prelude::*;
    use crate::util::DuoIndexMap;

    let mut map: DuoIndexMap<u32, &str, Vec<u32>> = Default::default();
    map.get_or_insert_default(1, || "one").unwrap().push(1);
    map.get_or_insert_default(2, || "two").unwrap().push(2);
    map.get_or_insert_default(1, || unreachable!()).unwrap().push(10);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get2("one"), Some(&vec![1, 10]));
    assert_eq!(map.get2("two"), Some(&vec![2]));
    assert_eq!(map.get2("three"), None);

    let mut keys: Vec<_> = map.secondary_keys().copied().collect();
    keys.sort();
    assert_eq!(keys, ["one", "two"]);

    let values: Vec<_> = map.par_iter().collect();
    assert_eq!(values, [&vec![1, 10], &vec![2]]);
}

#[test]
fn test_duo_index_map_duplicate_secondary_key() {
    use crate::util::DuoIndexMap;

    let mut map: DuoIndexMap<u32, &str, Vec<u32>> = Default::default();
    map.get_or_insert_default(1, || "one").unwrap().push(1);
    assert_eq!(map.get_or_insert_default(2, || "one"), Err("one"));

    assert_eq!(map.len(), 1);
    assert_eq!(map.get2("one"), Some(&vec![1]));

    // `2` was not inserted, so it's free to take a new key.
    assert_eq!(map.get_or_insert_default(2, || "two"), Ok(&mut vec![]));
}

#[test]
fn test_to_slash_lossy() {
    use std::path::Path;
    use crate::util::PathExt;

    assert_eq!(Path::new("").to_slash_lossy(), "");
    assert_eq!(Path::new("team").to_slash_lossy(), "team");
    assert_eq!(Path::new("2024/team").to_slash_lossy(), "2024/team");
    assert_eq!(Path::new("2024/team/").to_slash_lossy(), "2024/team");
    assert_eq!(Path::new("v1.2/team").to_slash_lossy(), "v1.2/team");

    #[cfg(windows)]
    assert_eq!(Path::new(r"2024\team").to_slash_lossy(), "2024/team");
}
