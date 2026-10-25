use super::*;
use std::path::Path;

fn environment() -> Environment<'static> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let tree = Arc::new(FsTree::build(root).unwrap());
    let globals: toml::Table = toml::from_str(
        "root = '/docs/'\ndate = 2024-04-05\nlabel = 'example'",
    ).unwrap();
    try_init(tree, None, globals).unwrap()
}

#[test]
fn template_helpers_and_globals() {
    let env = environment();
    let rendered = env.render_str(concat!(
        "{{ join('guide', 'start') }}|",
        "{{ G.date | date('%Y-%m-%d') }}|",
        "{{ G | get('label', 'fallback') }}|",
        "{{ G | get('missing', 'fallback') }}|",
        "{{ 'a:b:c' | split(':') | join(',') }}",
    ), ()).unwrap();
    assert_eq!(rendered, "/docs/guide/start|2024-04-05|example|fallback|a,b,c");
    assert!(env.render_str("{{ missing }}", ()).is_err());
}

#[test]
fn undefined_values() {
    let env = environment();
    assert_eq!(env.render_str(concat!(
        "{% if missing %}yes{% else %}no{% endif %}|",
        "{{ missing | default('fallback') }}",
    ), ()).unwrap(), "no|fallback");
    for template in [
        "{{ missing }}",
        "{{ missing.field }}",
        "{% for item in missing %}{{ item }}{% endfor %}",
        "{{ missing | split(':') }}",
    ] {
        assert!(env.render_str(template, ()).is_err(), "{template}");
    }
}

#[test]
fn metadata_and_list_values() {
    use crate::value::{Dict, List, Value as DataValue};

    let metadata = Metadata::new();
    metadata.insert_raw("title", "Example");
    metadata.insert_raw("nested", Dict::from([
        ("enabled", DataValue::Bool(false)),
        ("labels", DataValue::from(vec!["b", "a"])),
    ]));

    let items = List::default();
    for name in ["b", "a"] {
        let item = Arc::new(Metadata::new());
        item.insert_raw("name", name);
        items.push(item);
    }
    items.sort_by(|a, b| a.get_raw("name").cmp(&b.get_raw("name")));

    let mut env = environment();
    env.add_global("M", Value::from_object(metadata));
    env.add_global("items", Value::from_object(items));
    let rendered = env.render_str(concat!(
        "{{ M.title }}|{{ M.nested.labels[1] }}|",
        "{% for key in M.nested %}{{ key }},{% endfor %}|",
        "{% for item in items %}{{ loop.index }}={{ item.name }};{% endfor %}|",
        "{{ items[0].name }}|{{ items | length }}|",
        "{{ M.absent | default('fallback') }}",
    ), ()).unwrap();
    assert_eq!(rendered, "Example|a|enabled,labels,|1=a;2=b;|a|2|fallback");
    assert!(env.render_str("{{ M.absent }}", ()).is_err());
}

#[test]
#[cfg(feature = "plugins")]
fn lua_callbacks() {
    let env = environment();
    let rendered = env.render_str(concat!(
        "{{ 'hi' | repeat_text(2) }}|",
        "{{ record(42).value }}|",
        "{% if 3 is positive and not (-1 is positive) %}yes{% endif %}|",
        "{{ record({'labels': ['b', 'a']}).value.labels | join(',') }}",
    ), ()).unwrap();
    assert_eq!(rendered, "hihi|42|yes|b,a");
}
