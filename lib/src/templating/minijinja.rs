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

#[derive(Debug)]
pub struct SiteItem {
    pub site: Arc<Site>,
    pub collection: Option<Arc<Collection>>,
    pub item: Arc<Item>,
}

impl SiteItem {
    pub fn is_index(&self) -> bool {
        self.collection.as_ref()
            .and_then(|c| c.index.as_ref())
            .map_or(false, |i| i.entry.id == self.item.entry.id)
    }

    pub fn position(&self) -> Option<usize> {
        self.collection.as_ref()
            .and_then(|c| c.items.iter().position(|i| i.entry.id == self.item.entry.id))
    }
}

fn try_init<G: Serialize>(
    tree: Arc<FsTree>,
    root: Option<EntryId>,
    globals: G,
) -> Result<Environment<'static>> {
    let mut env = Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);

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
    env.add_filter("get", ext::get);
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

        Ok(template.render(Value::from_object(site_item))?)
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

        let context = Value::from_object(site_item);
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
        let context = Value::from_object(meta);
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
    use minijinja::{value::{intern, DynObject, Rest, Value}, Error, ErrorKind, State};

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
            let datetime = DateTime::from_timestamp(ts, 0)
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

    pub fn get(map: DynObject, key: &str, default: Value) -> Value {
        map.get_value(&Value::from(key)).unwrap_or(default)
    }
}

mod value_object {
    use std::fmt::Debug;
    use std::sync::Arc;
    use minijinja::value::{DynObject, Enumerator, Object, ObjectExt, ObjectRepr, Value};

    use crate::value;
    use crate::util::declare_variation;

    declare_variation!(Dict of value::Dict);

    impl Object for Dict {
        fn repr(self: &Arc<Self>) -> ObjectRepr {
            ObjectRepr::Map
        }

        fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
            self.0.get(key.as_str()?)
                .cloned()
                .map(Value::from)
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            self.mapped_rev_enumerator(|this| Box::new({
                this.keys().cloned().map(Value::from)
            }))
        }
    }

    impl<T: Clone + Debug + Into<DynObject> + Sync + Send> Object for value::List<T> {
        fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let item = self.get(key.as_usize()?)?.clone();
            Some(Value::from_dyn_object(item.into()))
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            Enumerator::Seq(Self::len(self))
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
                Value::Array(a) => Self::from_dyn_object(a),
                Value::Dict(d) => Self::from_dyn_object(Dict::new(d)),
            }
        }
    }
}

mod taxonomy_object {
    use std::{sync::Arc};
    use minijinja::value::{Enumerator, Object, ObjectExt, ObjectRepr, Value};

    use super::SiteItem;
    use crate::{declare_variation, taxonomy::{Collection, Item, Metadata, Site}, value::List};

    declare_variation!(SiteItems of Site);
    declare_variation!(SiteCollections of Site);
    declare_variation!(CollectionItems of Collection);
    declare_variation!(CollectionData of Collection);

    // FIXME: Use `this` or `item` to refer to the item to avoid key collisions
    // between our keys here and the keys in `self.item`.
    impl Object for SiteItem {
        fn repr(self: &Arc<Self>) -> ObjectRepr {
            ObjectRepr::Map
        }

        #[inline]
        fn get_value(self: &Arc<Self>, name: &Value) -> Option<Value> {
            let value = match name.as_str()? {
                "site" => Value::from_dyn_object(self.site.clone()),
                "collection" => Value::from_dyn_object(self.collection.as_ref()?.clone()),
                "position" => self.position()?.into(),
                "is_index" => self.is_index().into(),
                "next" => {
                    let collection = self.collection.as_ref()?;
                    let j = self.is_index()
                        .then_some(0)
                        .or_else(|| self.position().map(|i| i.saturating_add(1)))?;

                    let next = collection.items.get(j)?;
                    Value::from_dyn_object(next.clone())
                },
                "previous" => {
                    let collection = self.collection.as_ref()?;
                    let item = match self.position()? {
                        0 => collection.index.as_ref()?,
                        i => collection.items.get(i - 1)?,
                    };

                    Value::from_dyn_object(item.clone())
                }
                _ => self.item.get_value(name)?,
            };

            Some(value)
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            self.mapped_enumerator(|this| Box::new({
                let keys = &["site", "collection", "position", "is_index", "next", "previous"];
                let unique_keys = keys.into_iter()
                    .filter(|x| !this.item.metadata.contains_key(x))
                    .map(|x| Value::from(*x));

                this.item.metadata.fields().chain(unique_keys)
            }))
        }
    }

    impl Object for Site {
        fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let value = match key.as_str()? {
                "items" => Value::from_dyn_object(SiteItems::new(self.clone())),
                "collections" => Value::from_dyn_object(SiteCollections::new(self.clone())),
                _ => return None,
            };

            Some(value)
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            Enumerator::Str(&["items", "collections"])
        }
    }

    impl Object for SiteItems {
        fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let value = self.items.get(key.as_usize()?)?;
            Some(Value::from_dyn_object(value.clone()))
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            Enumerator::Seq(self.items.len())
        }
    }

    impl Object for SiteCollections {
        fn repr(self: &Arc<Self>) -> ObjectRepr {
            ObjectRepr::Map
        }

        fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let id = self.index.get(key.as_str()?)?;
            let collection = self.collections.get(id)?.clone();
            Some(Value::from_dyn_object(collection))
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            self.mapped_enumerator(|this| Box::new({
                this.index.keys().map(|k| Value::from(k.clone()))
            }))
        }
    }

    impl Object for Collection {
        fn get_value(self: &Arc<Self>, name: &Value) -> Option<Value> {
            Some(match name.as_str()? {
                "index" => Value::from_dyn_object(self.index.clone()?),
                "items" => Value::from_dyn_object(CollectionItems::new(self.clone())),
                "data" => Value::from_dyn_object(CollectionData::new(self.clone())),
                _ => return None,
            })
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            Enumerator::Str(&["index", "items", "data"])
        }
    }

    impl Object for CollectionItems {
        fn get_value(self: &Arc<Self>, value: &Value) -> Option<Value> {
            let item = self.items.get(value.as_usize()?)?;
            Some(Value::from_dyn_object(item.clone()))
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            Enumerator::Seq(List::len(&self.items))
        }
    }

    impl Metadata {
        fn get_value(&self, name: &Value) -> Option<Value> {
            self.get_raw(name.as_str()?).map(Value::from)
        }

        fn fields(&self) -> impl Iterator<Item = Value> + '_ {
            self.keys().map(Value::from)
        }
    }

    impl Object for Metadata {
        fn repr(self: &Arc<Self>) -> ObjectRepr {
            ObjectRepr::Map
        }

        fn get_value(self: &Arc<Self>, name: &Value) -> Option<Value> {
            Metadata::get_value(self, name)
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            self.mapped_enumerator(|this| Box::new(this.fields()))
        }
    }

    impl Object for Item {
        fn repr(self: &Arc<Self>) -> ObjectRepr {
            ObjectRepr::Map
        }

        fn get_value(self: &Arc<Self>, name: &Value) -> Option<Value> {
            self.metadata.get_value(name)
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            self.mapped_enumerator(|this| Box::new(this.metadata.fields()))
        }
    }

    impl Object for CollectionData {
        fn repr(self: &Arc<Self>) -> ObjectRepr {
            ObjectRepr::Map
        }

        fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
            let name = key.as_str()?;
            let id = self.data.keys()
                .find(|id| self.entry.tree[**id].file_stem() == name)?;

            let list = self.data.get(id)?.clone();
            Some(Value::from_dyn_object(list))
        }

        fn enumerate(self: &Arc<Self>) -> Enumerator {
            self.mapped_enumerator(|this| Box::new({
                this.data.keys()
                    .map(|id| this.entry.tree[*id].file_stem())
                    .map(Value::from)
            }))
        }
    }
}

impl_error_detail_with_std_error!(minijinja::Error);
