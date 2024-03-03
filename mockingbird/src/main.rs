use std::{path::Path, sync::Arc};

use harper::Renderer;
use harper::value::Value;
use harper::path_str::PathStr;
use harper::url::Url;

use crate::discover::Mockingbird;

mod config;
mod discover;
mod render;
mod util;

pub const CONTENT_DIR: &str = "content";
pub const TEMPLATE_DIR: &str = "templates";
pub const ASSETS_DIR: &str = "assets";
pub const CONFIG_FILE: &str = "config.toml";

harper::define_meta_key! {
    pub UrlRef : "url" => Arc<Url>,
    pub PermaPath : "permapath" => Arc<Path>,
    pub Template : "template" => Arc<PathStr>,
    pub Slug : "slug" => Arc<str>,

    pub SourcePath : "source_path" => Arc<Path>,
    pub FileStem : "file_stem" => Arc<str>,

    pub Position : "position" => usize,
    pub Draft : "draft" => bool,

    pub Content : "content" => Arc<str>,
    pub Data : "data" => Value,

    pub Toc : "toc" => Arc<str>,
    pub Snip : "snippet" => Arc<str>,
}

pub fn main() {
    use std::path::PathBuf;
    use harper::templating::minijinja::MiniJinjaEngine;

    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().expect("<input>"));
    let output = PathBuf::from(args.next().expect("<output>"));

    let start = std::time::SystemTime::now();
    harper::markdown::SyntaxHighlight::warm_up();
    let result = Mockingbird::new::<MiniJinjaEngine, _, _>(input, output)
        .and_then(|mockingbird| Ok((mockingbird.discover()?, mockingbird)))
        .and_then(|(site, mockingbird)| {
            let site = Arc::new(site);
            println!("discovery time: {}ms", start.elapsed().unwrap().as_millis());
            let render = std::time::SystemTime::now();
            let result = mockingbird.render_site(&site);
            println!("render time: {}ms", render.elapsed().unwrap().as_millis());
            println!("total time: {}ms", start.elapsed().unwrap().as_millis());
            site.visualize();
            result
        });

    if let Err(e) = result {
        println!("error: {e}");
    }
}
