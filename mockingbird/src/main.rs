use std::sync::Arc;
use std::path::Path;

use harper::{Renderer, Site};
use harper::error::Result;
use harper::value::Value;
use harper::path_str::PathStr;
use harper::templating::minijinja::MiniJinjaEngine;
use harper::url::Url;

#[macro_use]
mod util;
mod config;
mod discover;
mod render;

use crate::discover::Mockingbird;

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

pub fn run(input: &Path, output: &Path) -> Result<Arc<Site>> {
    let mockingbird = Mockingbird::new::<MiniJinjaEngine, _, _>(input, output)?;
    let site = Arc::new(mockingbird.discover()?);
    mockingbird.render_site(&site)?;
    Ok(site)
}

mod flags {
    use std::path::PathBuf;

    xflags::xflags! {
        /// Your friendly neighborhood bird.
        cmd mockingbird {
            /// Build a site.
            default cmd build {
                /// Directory containing the site sources
                required input: PathBuf
                /// Where to write the site to
                required output: PathBuf
                /// quiet: don't emit anything
                optional -q,--quiet
            }
            /// Print the version and exit.
            cmd version { }
        }
    }
}

pub fn main() {
    harper::markdown::SyntaxHighlight::warm_up();

    match flags::Mockingbird::from_env_or_exit().subcommand {
        flags::MockingbirdCmd::Build(args) => {
            let site = run(&args.input, &args.output).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1)
            });

            if !args.quiet {
                site.visualize();
            }
        }
        flags::MockingbirdCmd::Version(_) => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        }
    }
}
