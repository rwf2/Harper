use std::path::Path;

use harper::Site;
use harper::value::List;
use harper::templating::minijinja::MiniJinjaEngine;

use crate::discover::Mockingbird;

fn discover(fixture: &str) -> Site {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(fixture);
    let mockingbird = Mockingbird::new::<MiniJinjaEngine, _, _>(&root, root.join("out")).unwrap();
    mockingbird.discover().unwrap()
}

fn sorted<'a, T: AsRef<str> + 'a>(keys: impl Iterator<Item = &'a T>) -> Vec<&'a str> {
    let mut keys: Vec<_> = keys.map(|k| k.as_ref()).collect();
    keys.sort();
    keys
}

#[test]
fn data_named_by_path_relative_to_collection() {
    let site = discover("data");
    assert_eq!(sorted(site.collections.secondary_keys()), ["/", "blog"]);

    let root = site.collections.get2("/").unwrap();
    assert!(root.index.is_some());

    let blog = site.collections.get2("blog").unwrap();
    assert!(blog.index.is_some());
    assert_eq!(List::len(&blog.items), 1);
    assert_eq!(sorted(blog.data.secondary_keys()), ["2024/team", "2025/team", "authors", "v1.2"]);
    assert_eq!(List::len(blog.data.get2("authors").unwrap()), 2);
    assert_eq!(List::len(blog.data.get2("2024/team").unwrap()), 1);
    assert_eq!(List::len(blog.data.get2("2025/team").unwrap()), 1);
}

#[test]
fn root_collection_without_index() {
    let site = discover("no-root-index");
    assert_eq!(sorted(site.collections.secondary_keys()), ["/", "blog"]);

    let root = site.collections.get2("/").unwrap();
    assert!(root.index.is_none());
    assert_eq!(List::len(&root.items), 1);
}

#[test]
fn collection_name_conflict_is_an_error() {
    let mut site = discover("data");
    let blog = site.collections.get2("blog").unwrap().entry.id;
    let authors = site.tree[blog].children.iter()
        .copied()
        .find(|&id| site.tree[id].file_name == "authors")
        .unwrap();

    let error = site.get_or_insert_collection(|| "blog".into(), authors).unwrap_err();
    assert!(error.to_string().contains("multiple collections have the same name"));
    assert!(site.collections.get2("blog").is_some_and(|c| c.entry.id == blog));
}
