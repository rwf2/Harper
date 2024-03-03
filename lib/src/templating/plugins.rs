use std::sync::Arc;

use either::Either;
use mlua::{Lua, Value, Function, Table, LuaSerdeExt, MultiValue};
use minijinja::value::Value as TemplateValue;
use thread_local::ThreadLocal;

use crate::fstree::FsTree;
use crate::error::{Result, Error, ErrorDetail};
use crate::value::Source;

pub struct LazyThreadLocal<T: Send> {
    tls: ThreadLocal<T>,
    init_fn: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T: Send> LazyThreadLocal<T> {
    pub fn new<F: Fn() -> T>(init_fn: F) -> LazyThreadLocal<T>
        where F: Send + Sync + 'static
    {
        LazyThreadLocal {
            tls: ThreadLocal::new(),
            init_fn: Box::new(init_fn),
        }
    }

    pub fn get(&self) -> &T {
        self.tls.get_or(|| (self.init_fn)())
    }
}

pub struct PluginContext {
    lua: LazyThreadLocal<mlua::Result<mlua::Lua>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Callback {
    Filter,
    Function,
    Test
}

impl Callback {
    pub fn key(&self) -> &'static str {
        match self {
            Callback::Filter => "filters",
            Callback::Function => "functions",
            Callback::Test => "tests",
        }
    }
}

impl PluginContext {
    fn lua(&self) -> Result<&mlua::Lua> {
        self.lua.get()
            .as_ref()
            .map_err(|e| Error::from_detail(e))
    }

    fn api(&self) -> Result<mlua::Table> {
        let lua = self.lua()?;
        Ok(lua.globals().get("harper")?)
    }

    pub fn callbacks(&self) -> Result<Vec<(Callback, String)>> {
        let mut list = vec![];
        let api = self.api()?;
        for callback in [Callback::Filter, Callback::Function, Callback::Test] {
            let callbacks: Table = api.get(callback.key())?;
            for pair in callbacks.pairs::<String, Value>() {
                let (key, _) = pair?;
                list.push((callback, key));
            }
        }

        Ok(list)
    }

    pub fn call<O: TryFrom<TemplateValue>>(
        &self,
        kind: Callback,
        name: &str,
        args: Vec<TemplateValue>
    ) -> Result<O>
        where O::Error: ErrorDetail + 'static,
    {
        let callbacks: Table = self.api()?.get(kind.key())?;
        let callback: Function = callbacks.get(name)?;

        let lua = self.lua()?;
        let values = args.iter()
            .map(|v| lua.to_value(v))
            .collect::<mlua::Result<Vec<Value>>>()?;

        let raw: Value = callback.call(MultiValue::from_vec(values))?;
        let value = TemplateValue::from_serializable(&raw);
        let value = value.try_into()?;
        Ok(value)
    }
}

pub fn lua(chunk: &str, name: &str) -> mlua::Result<Lua> {
    let lua = Lua::new();

    // setup the API
    lua.load(r#"
        harper = {
            filters = {},
            functions = {},
            tests = {},
        }

        function harper.register_filter(name, func)
            harper.filters[name] = func
        end

        function harper.register_function(name, func)
            harper.functions[name] = func
        end

        function harper.register_test(name, func)
            harper.tests[name] = func
        end
    "#).exec()?;

    lua.load(&*chunk).set_name(&*name).exec()?;
    Ok(lua)
}

pub fn init(tree: Arc<FsTree>) -> Result<Option<PluginContext>> {
    let file = match tree.get(tree.root_id(), "plugins/init.lua") {
        Some(file) => file,
        None => return Ok(None)
    };

    let chunk = match file.path.as_ref().read()? {
        Either::Left(string) => string,
        _ => return err!(
            "init.lua contained invalid UTF-8",
            "full path" => file.path.display(),
        ),
    };

    let name = file.path
        .strip_prefix(&tree.root().path)
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let lua = LazyThreadLocal::new(move || lua(&*chunk, &*name));
    Ok(Some(PluginContext { lua }))
}

impl_error_detail_with_std_error!(mlua::Error);

// struct LuaFilter<'lua> {
//     name: &'lua str,
//     function: mlua::Function<'lua>,
// }

// impl LuaPluginEnvironment {
//     // fn lua(&self) -> impl Deref<Target=mlua::Lua> + '_ {
//     //     ReentrantMutexGuard::map(self.0.lock(), |ctxt| &*ctxt.lua)
//     // }
//
//     fn new() -> Self {
//         let env = LuaPluginEnvironment(Arc::new(Mutex::new(LuaPluginContext {
//             lua: Box::pin(mlua::Lua::new()),
//         })));
//
//         let globals = env.clone();
//         let globals = globals.0.lock();
//         let globals = globals.lua.globals();
//         globals.set("harper", env.clone());
//         env
//     }
// }
//
// impl UserData for LuaPluginEnvironment {
//     fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
//         methods.add_method("register_filter", |_, ctxt, (name, func): (mlua::String<'_>, mlua::Function<'_>)| {
//             let (name, func) = unsafe {
//                 let name: &'static str = std::mem::transmute(name.to_str()?);
//                 let func: mlua::Function<'static> = std::mem::transmute(func);
//                 (name, func)
//             };
//
//             ctxt.0.filters.insert(name, func);
//             Ok(())
//         });
//     }
// }
//
// struct JinjaValue(TemplateValue);
//
// impl<'lua> ToLua<'lua> for JinjaValue {
//     fn to_lua(self, lua: &'lua Lua) -> mlua::Result<Value<'lua>> {
//         lua.to_value(&self.0)
//     }
// }
//
// impl<'lua> FromLua<'lua> for JinjaValue {
//     fn from_lua(value: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
//         Ok(JinjaValue(TemplateValue::from_serializable(&value)))
//     }
// }
//
// struct SketchLua;
//
// impl mlua::UserData for SketchLua {
//     fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
//         methods.add_method("register_filter", |lua, v, f: mlua::Function<'lua>| {
//             Ok(f)
//         })
//     }
// }
