use std::sync::Arc;
use minijinja::{Environment, path_loader};
use minijinja::value::Value;
use serde::Serialize;

use crate::taxonomy::{Site, Item, Collection, Metadata};
use crate::error::Result;
use crate::fstree::{FsTree, EntryId};
use crate::templating::{Engine, EngineInit};

#[derive(Debug)]
pub struct MiniJinjaEngine {
    env: Result<Environment<'static>>,
}

pub struct SiteItem {
    pub site: Arc<Site>,
    pub collection: Option<Arc<Collection>>,
    pub item: Arc<Item>,
}

impl SiteItem {
    pub fn is_index(&self) -> bool {
        self.collection.as_ref()
            .and_then(|c| c.index.as_ref())
            .map_or(false, |i| i.id == self.item.id)
    }

    pub fn position(&self) -> Option<usize> {
        self.collection.as_ref()
            .and_then(|c| c.items.iter().position(|i| i.id == self.item.id))
    }
}

fn try_init<G: Serialize>(
    tree: Arc<FsTree>,
    root: Option<EntryId>,
    globals: G,
) -> Result<Environment<'static>> {
    let mut env = Environment::new();
    if let Some(root) = root {
        env.set_loader(path_loader(&tree[root].path));
    }

    #[cfg(feature = "plugins")]
    if let Some(plugins) = super::plugins::init(tree)? {
        use minijinja::State;
        use minijinja::value::Rest;

        let plugins = Arc::new(plugins);
        for (kind, name) in plugins.callbacks()? {
            let plugins = plugins.clone();
            match kind {
                crate::templating::plugins::Callback::Filter => {
                    env.add_filter(name.clone(), move |_: &State, values: Rest<Value>| {
                        plugins.call::<Value>(super::plugins::Callback::Filter, &*name, values.0)
                            .map_err(|e| minijinja::Error::new(
                                minijinja::ErrorKind::InvalidOperation,
                                format!("lua plugin error:\n{e}")
                            ))
                    });
                },
                super::plugins::Callback::Function => {
                    env.add_function(name.clone(), move |_: &State, values: Rest<Value>| {
                        plugins.call::<Value>(super::plugins::Callback::Function, &*name, values.0)
                            .map_err(|e| minijinja::Error::new(
                                minijinja::ErrorKind::InvalidOperation, e.to_string()
                            ))
                    });
                },
                super::plugins::Callback::Test => {
                    env.add_test(name.clone(), move |_: &State, value: Value| {
                        plugins.call::<bool>(super::plugins::Callback::Test, &*name, vec![value])
                            .map_err(|e| minijinja::Error::new(
                                minijinja::ErrorKind::InvalidOperation, e.to_string()
                            ))
                    });
                }
            }
        }
    }

    env.add_global("G", Value::from_serializable(&globals));
    env.add_function("join", ext::join);
    env.add_function("now", ext::now);
    env.add_filter("deslug", ext::deslug);
    env.add_filter("date", ext::date);
    env.add_filter("split", ext::split);
    Ok(env)
}

impl EngineInit for MiniJinjaEngine {
    type Engine = Self;

    fn init<G: Serialize>(tree: Arc<FsTree>, root: Option<EntryId>, globals: G) -> Self::Engine {
        MiniJinjaEngine { env: try_init(tree, root, globals) }
    }
}

impl Engine for MiniJinjaEngine {
    fn render(
        &self,
        name: &str,
        site: &Arc<Site>,
        collection: Option<&Arc<Collection>>,
        item: &Arc<Item>,
    ) -> Result<String> {
        let env = self.env.as_ref().map_err(|e| e.clone())?;
        let template = env.get_template(name)?;
        let site_item = SiteItem {
            site: site.clone(),
            collection: collection.cloned(),
            item: item.clone()
        };

        Ok(template.render(Value::from_map_object(site_item))?)
    }

    fn render_raw(
        &self,
        name: Option<&str>,
        template_str: &str,
        site: &Arc<Site>,
        collection: Option<&Arc<Collection>>,
        item: &Arc<Item>,
    ) -> Result<String> {
        let env = self.env.as_ref().map_err(|e| e.clone())?;
        let site_item = SiteItem {
            site: site.clone(),
            collection: collection.cloned(),
            item: item.clone()
        };

        let context = Value::from_map_object(site_item);
        let string = match name {
            Some(name) => env.render_named_str(name, template_str, context)?,
            None => env.render_str(template_str, context)?,
        };

        Ok(string)
    }

    fn render_str(
        &self,
        name: Option<&str>,
        template_str: &str,
        meta: Metadata,
    ) -> Result<String> {
        let env = self.env.as_ref().map_err(|e| e.clone())?;
        let context = Value::from_map_object(meta);
        let string = match name {
            Some(name) => env.render_named_str(name, template_str, context)?,
            None => env.render_str(template_str, context)?,
        };

        Ok(string)
    }
}

mod ext {
    use std::sync::Arc;

    use chrono::{NaiveDate, NaiveTime, DateTime, Utc};
    use minijinja::{value::{intern, Rest, Value}, Error, ErrorKind, State};

    use crate::url::Url;

    trait Ext {
        fn find(self, key: &str) -> Result<Value, Error>;
    }

    impl Ext for Value {
        fn find(self, key: &str) -> Result<Value, Error> {
            if key.is_empty() {
                return Ok(self);
            }

            let mut value = self;
            for attr in key.split('.') {
                let attr = value.get_attr(attr)?;

                if attr.is_undefined() {
                    return Err(Error::new(
                        ErrorKind::UndefinedError,
                        format!("missing key {key} in {value:#?}")
                    ));
                }

                value = attr;
            }

            Ok(value)
        }
    }

    impl Ext for &State<'_, '_> {
        fn find(self, key: &str) -> Result<Value, Error> {
            let (base, key) = key.split_once('.').unwrap_or((key, ""));
            let base_val = self.lookup(base)
                .filter(|v| !v.is_undefined())
                .ok_or_else(|| Error::new(
                    ErrorKind::MissingArgument,
                    format!("expected {base} in context but it wasn't found")
                ))?;

            base_val.find(key)
        }
    }

    // FIXME: Call this `url`. But that means revamping the SiteItem as in
    // minijinja2 (this.url, so namespace doesn't contain `url`).
    pub fn join<'a>(state: &'a State<'a, 'a>, values: Rest<Arc<str>>) -> Result<Value, Error> {
        let url_base = state.find("G.root")?;
        let url_base = url_base.as_str()
            .and_then(Url::try_new)
            .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "G.root must be a URL"))?;

        let mut url = url_base.to_url_buf();
        url.extend(values.iter());
        Ok(Value::from_safe_string(url.into()))
    }

    pub fn deslug(value: &str) -> String {
        value.replace('-', " ")
    }

    pub fn date(value: Value, fmt: &str) -> Result<Value, Error> {
        use chrono::naive::NaiveDateTime;

        if let Ok(ts) = value.clone().try_into() {
            let datetime = NaiveDateTime::from_timestamp_opt(ts, 0)
                .ok_or_else(|| Error::new(
                    ErrorKind::InvalidOperation,
                    "invalid timestamp provided to `date`"
                ))?;

            return Ok(datetime.format(fmt).to_string().into());
        }

        let kind = value.kind();
        let attr = value.get_attr("$__toml_private_datetime");
        let string = attr.as_ref()
            .map_or_else(|_| value.as_str(), |v| v.as_str())
            .ok_or_else(|| Error::new(
                ErrorKind::InvalidOperation,
                format!("`date` must be applied to a string or integer, found {kind}")
            ))?;

        let datetime = string.parse::<NaiveDate>().map(|d| d.format(fmt))
            .or_else(|_| string.parse::<NaiveTime>().map(|t| t.format(fmt)))
            .or_else(|_| string.parse::<NaiveDateTime>().map(|dt| dt.format(fmt)))
            .or_else(|_| string.parse::<DateTime<Utc>>().map(|dt| dt.format(fmt)))
            .map_err(|e| Error::new(
                ErrorKind::InvalidOperation,
                format!("failed to parse {string}: {e}")
            ))?;

        Ok(datetime.to_string().into())
    }

    pub fn split(value: &str, pat: &str, n: Option<usize>) -> Result<Value, Error> {
        match n {
            Some(n) => Ok(value.split(pat).nth(n).map(Value::from).unwrap_or(Value::UNDEFINED)),
            None => Ok(value.split(pat).map(intern).collect()),
        }
    }

    pub fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

mod value_object {
    use std::sync::Arc;
    use minijinja::value::{AnyMapObject, MapObject, SeqObject, Value};

    use crate::value;
    use crate::util::declare_variation;

    declare_variation!(Dict of value::Dict);
    declare_variation!(Array of Vec<value::Value>);

    impl MapObject for Dict {
        fn get_field(self: &Arc<Self>, key: &Value) -> Option<Value> {
            self.0.get(key.as_str()?)
                .cloned()
                .map(Value::from)
        }

        fn fields(self: &Arc<Self>) -> Vec<Value> {
            self.0.keys()
                .cloned()
                .map(Value::from)
                .collect()
        }

        fn field_count(self: &Arc<Self>) -> usize {
            self.0.len()
        }
    }

    impl SeqObject for Array {
        fn get_item(self: &Arc<Self>, idx: usize) -> Option<Value> {
            self.0.get(idx)
                .cloned()
                .map(Value::from)
        }

        fn item_count(self: &Arc<Self>) -> usize {
            self.0.len()
        }
    }

    impl<T: Clone + Into<AnyMapObject>> SeqObject for value::List<T> {
        fn get_item(self: &Arc<Self>, idx: usize) -> Option<Value> {
            let item = self.get(idx)?.clone();
            Some(Value::from(item.into()))
        }

        fn item_count(self: &Arc<Self>) -> usize {
            self.len()
        }
    }

    impl From<value::Value> for Value {
        fn from(value: crate::value::Value) -> Self {
            use crate::value::Value;

            match value {
                Value::Null => Self::UNDEFINED,
                Value::Bool(b) => Self::from(b),
                Value::Num(n) => match n.to_u128_strict() {
                    Ok(v) => Self::from(v),
                    Err(v) => Self::from(v),
                },
                Value::String(s) => Self::from(s),
                Value::Path(s) => Self::from(s.into::<Arc<str>>()),
                Value::Array(a) => Self::from_any_seq_object(Array::new(a)),
                Value::Dict(d) => Self::from_any_map_object(Dict::new(d)),
            }
        }
    }
}

mod taxonomy_object {
    use std::{path::Path, sync::Arc};
    use minijinja::value::{intern, MapObject, SeqObject, Value};

    use super::SiteItem;
    use crate::{declare_variation, taxonomy::{Collection, Item, Metadata, Site}};

    declare_variation!(SiteCollections of Site);
    declare_variation!(CollectionData of Collection);

    impl MapObject for SiteItem {
        #[inline]
        fn get_field(self: &Arc<Self>, name: &Value) -> Option<Value> {
            let value = match name.as_str()? {
                "site" => Value::from_any_map_object(self.site.clone()),
                "collection" => Value::from_any_map_object(self.collection.as_ref()?.clone()),
                "position" => self.position()?.into(),
                "is_index" => self.is_index().into(),
                "next" => {
                    let collection = self.collection.as_ref()?;
                    let j = self.is_index()
                        .then_some(0)
                        .or_else(|| self.position().map(|i| i.saturating_add(1)))?;

                    let next = collection.items.get(j)?;
                    Value::from_any_map_object(next.clone())
                },
                "previous" => {
                    let collection = self.collection.as_ref()?;
                    let item = match self.position()? {
                        0 => collection.index.as_ref()?,
                        i => collection.items.get(i - 1)?,
                    };

                    Value::from_any_map_object(item.clone())
                }
                _ => self.item.get_field(name)?,
            };

            Some(value)
        }

        fn fields(self: &Arc<Self>) -> Vec<Value> {
            let mut item_keys = self.item.fields();
            // TODO: if the item already contains fields with any of the names
            // below, we're going to duplicate them. is that okay?
            item_keys.extend_from_slice(&[
                intern("site").into(),
                intern("collection").into(),
                intern("position").into(),
                intern("is_index").into(),
                intern("next").into(),
                intern("previous").into(),
            ]);

            item_keys
        }

        fn field_count(self: &Arc<Self>) -> usize {
            self.item.field_count() + 6
        }
    }

    impl MapObject for Site {
        fn get_field(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let value = match key.as_str()? {
                "items" => Value::from_any_seq_object(self.clone()),
                "collections" => Value::from_any_map_object(SiteCollections::new(self.clone())),
                _ => return None,
            };

            Some(value)
        }

        fn static_fields(&self) -> Option<&'static [&'static str]> {
            Some(&["items", "collections"])
        }
    }

    impl SeqObject for Site {
        fn get_item(self: &Arc<Self>, idx: usize) -> Option<Value> {
            Some(Value::from_any_map_object(self.items.get(idx)?.clone()))
        }

        fn item_count(self: &Arc<Self>) -> usize {
            self.items.len()
        }
    }

    impl MapObject for SiteCollections {
        fn get_field(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let name = key.as_str()?;
            let id = self.collections.keys()
                .find(|id| self.tree[**id].relative_path() == Path::new(name))?;

            let collection = self.collections.get(id)?.clone();
            Some(Value::from_any_map_object(collection))
        }

        fn fields(self: &Arc<Self>) -> Vec<Value> {
            self.collections.values()
                .map(|c| c.root_entry().relative_path())
                .filter_map(|p| p.to_str())
                .map(Value::from)
                .collect()
        }

        fn field_count(self: &Arc<Self>) -> usize {
            self.collections.len()
        }
    }

    impl MapObject for Collection {
        fn get_field(self: &Arc<Self>, name: &Value) -> Option<Value> {
            let value = match name.as_str()? {
                "index" => Value::from_any_map_object(self.index.clone()?),
                "items" => Value::from_any_seq_object(self.clone()),
                "data" => Value::from_any_map_object(CollectionData::new(self.clone())),
                _ => return None,
            };

            Some(value)
        }

        fn static_fields(&self) -> Option<&'static [&'static str]> {
            Some(&["index", "items", "data"])
        }
    }

    impl SeqObject for Collection {
        fn get_item(self: &Arc<Self>, idx: usize) -> Option<Value> {
            Some(Value::from_any_map_object(self.items.get(idx)?.clone()))
        }

        fn item_count(self: &Arc<Self>) -> usize {
            self.items.len()
        }
    }

    impl Metadata {
        fn get_field(&self, name: &Value) -> Option<Value> {
            self.get_raw(name.as_str()?).map(Value::from)
        }

        fn fields(&self) -> Vec<Value> {
            self.keys()
                .map(Value::from)
                .collect()
        }
    }

    impl MapObject for Metadata {
        #[inline]
        fn get_field(self: &Arc<Self>, name: &Value) -> Option<Value> {
            Metadata::get_field(self, name)
        }

        #[inline(always)]
        fn fields(self: &Arc<Self>) -> Vec<Value> {
            Metadata::fields(self)
        }

        fn field_count(self: &Arc<Self>) -> usize {
            self.len()
        }
    }

    impl MapObject for Item {
        fn get_field(self: &Arc<Self>, name: &Value) -> Option<Value> {
            match name.as_str() {
                Some("id") => Some(self.id.0.into()),
                _ => self.metadata.get_field(name)
            }
        }

        fn fields(self: &Arc<Self>) -> Vec<Value> {
            // FIXME: What if `id` already in `self.metadata`?
            let mut fields = self.metadata.fields();
            fields.push("id".into());
            fields
        }

        fn field_count(self: &Arc<Self>) -> usize {
            // FIXME: What if `id` already in `self.metadata`?
            self.metadata.len() + 1
        }
    }

    impl MapObject for CollectionData {
        fn get_field(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let name = key.as_str()?;
            let id = self.data.keys()
                .find(|id| self.tree[**id].file_stem() == name)?;

            let list = self.data.get(id)?.clone();
            Some(Value::from_any_seq_object(list))
        }

        fn fields(self: &Arc<Self>) -> Vec<Value> {
            self.data.keys()
                .map(|id| self.tree[*id].file_stem())
                .map(intern)
                .map(Value::from)
                .collect()
        }

        fn field_count(self: &Arc<Self>) -> usize {
            self.data.len()
        }
    }

}

impl_error_detail_with_std_error!(minijinja::Error);
