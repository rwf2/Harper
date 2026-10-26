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
fn sequences() {
    use crate::taxonomy::Site;
    use crate::value::Value as DataValue;

    let env = environment();
    let tree = Arc::new(FsTree::build(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")).unwrap());
    let file = tree.get_file_id(tree.root_id(), "plugins/init.lua").unwrap();

    let mut site = Site::new(tree.clone());
    let collection = site.get_or_insert_collection(|| "c".into(), tree.root_id()).unwrap();
    for name in ["a", "b", "c"] {
        collection.new_item(file).metadata.insert_raw("name", name);
    }

    for name in ["a", "b", "c"] {
        site.new_resource(file).metadata.insert_raw("name", name);
    }

    let site = Arc::new(site);
    let collection = site.collections.get2("c").unwrap().clone();
    let context = minijinja::context! {
        items => Value::from_dyn_object(collection),
        site => Value::from_dyn_object(site),
        array => Value::from(DataValue::from(vec!["a", "b", "c"])),
    };

    for seq in ["items.items", "site.items"] {
        let rendered = env.render_str(&format!(concat!(
            "{{{{ {0}[:2] | map(attribute='name') | join(',') }}}}|",
            "{{{{ {0}[-1].name }}}}|{{{{ ({0} | last).name }}}}|",
            "{{{{ ({0} | reverse | first).name }}}}|{{{{ {0} | length }}}}|",
            "{{% if {0} is sequence and {0} is not mapping %}}seq{{% endif %}}|",
            "{{% if {0}[1] in {0} %}}in{{% endif %}}",
        ), seq), &context).unwrap();
        assert_eq!(rendered, "a,b|c|c|c|3|seq|in", "{seq}");
    }

    let rendered = env.render_str(concat!(
        "{{ array[1:] | join(',') }}|{{ array[-1] }}|{{ array | last }}|",
        "{% if 'b' in array and 'z' not in array %}in{% endif %}",
    ), &context).unwrap();
    assert_eq!(rendered, "b,c|c|c|in");
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
