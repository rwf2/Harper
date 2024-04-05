use std::sync::Arc;
use std::path::{PathBuf, Path};
use std::borrow::Cow;

use harper::rayon::prelude::*;
use harper::url::UrlBuf;
use harper::error::{Result, Chainable};
use harper::{error, render_site, Collection, Site};
use harper::{Item, Kind, Renderer};
use harper::value::{Grass, Json, Mapper, Sink, Source, Toml};
use harper::markdown::{self, *};
use harper::path_str::IntoPathStrLossy;

use crate::util::{StringExt, ValueExt};
use crate::{Content, Draft, PermaPath, Slug, Snip, Template, Toc, UrlRef};
use crate::discover::Mockingbird;

impl Renderer for Mockingbird {
    type Output = ();
    type Collected = ();
    type Render = ();

    fn render_site(&self, site: &Arc<Site>) -> Result<Self::Output> {
        render_site(self, site)?;

        site.collections.par_iter().map(|(_, collection)| collection.par_map_items(|_, item| {
            // TODO: Validate template path? TODO: Validate permapath?
            let Some(Ok(permapath)) = item.metadata.get(PermaPath) else {
                return Ok(());
            };

            let output = self.output.join(permapath);
            std::fs::create_dir_all(output.parent().unwrap())?;

            match item.metadata.get(Template) {
                Some(Err(e)) => return Err(e.type_err(Template, "invalid template value")),
                Some(Ok(template)) => {
                    output.write(self.config.engine
                        .render(template.as_str(), site, Some(collection), item)
                        .chain_with(|| error! {
                            "failed to render item",
                            "path" => item.entry.relative_path().display(),
                            "template used" => template.as_str(),
                        })?)
                },
                None => {
                    let content: Arc<str> = item.entry.try_read()?;
                    if !harper::util::is_template(&*content) {
                        return output.write(content);
                    }

                    let name = item.entry.relative_path().to_string_lossy();
                    output.write(self.config.engine
                        .render_raw(Some(&*name), &content, site, Some(collection), item)
                        .chain_with(|| error! {
                            "failed to render direct item",
                            "path" => name,
                        })?)
                }
            }
        })).collect()
    }

    // TODO: We would like to be able to templatize JSON too.
    fn render_collection_item(&self,
        kind: Kind,
        _: &Arc<Site>,
        collection: &Arc<Collection>,
        item: &Arc<Item>
    ) -> Result<Self::Render> {
        const KNOWN_EXTS: &[&str] = &["md", "mdown", "markdown", "toml", "json"];

        if let Some(Ok(true)) = item.metadata.get(Draft) {
            return Ok(());
        }

        let entry = &*item.entry;
        match entry.file_ext() {
            Some("md") | Some("mdown") | Some("markdown") => {
                let engine = self.config.engine.clone();
                Markdown::from(entry)
                    .plugin(FrontMatter::new(Toml, &item.metadata))
                    .plugin(Templatize::with(entry.relative_path(), engine, &item.metadata))
                    .plugin(Alias::new(&self.config.settings.aliases))
                    .plugin(AutoHeading::default())
                    .plugin(TableOfContents::new(item.metadata.metakey(Toc)))
                    .plugin(Snippet::new(item.metadata.metakey(Snip), 250))
                    .plugin(Admonition::default())
                    .plugin(AutoHeading::default())
                    .plugin(HeadingAnchor::default())
                    .plugin(CodeTrim::trim(|l, _| l.trim().starts_with("# ") || l.trim() == "#"))
                    .plugin(CodeTrim::trim_start())
                    .plugin(Alias::new(&self.config.settings.aliases))
                    // .plugin(TsHighligher::default())
                    .plugin(SyntaxHighlight::default())
                    .plugin(Parts::new(item.metadata.key("parts")))
                    .plugin(markdown::Renderer::new(item.metadata.metakey(Content)))
                    .run()
                    .chain_with(|| "markdown rendering failed")?;
            },
            Some("toml") => Toml.map_copy(entry, &item.metadata).chain_with(|| error! {
                "TOML deserialization failed",
                "path" => entry.relative_path().display()
            })?,
            Some("json") => Json.map_copy(entry, &item.metadata).chain_with(|| error! {
                "JSON deserialization failed",
                "path" => entry.relative_path().display()
            })?,
            _ => { }
        };

        // Computte the permapath and Url.
        let content_root = &self.tree[self.content_root];
        let group_perma = collection.entry.path_relative_to(content_root).unwrap();
        let rendered = entry.file_ext().map_or(false, |e| KNOWN_EXTS.contains(&e));
        let slug = item.metadata
            .get_or_insert_with(Slug, || item.entry.file_stem().slugify())
            .map_err(|v| v.type_err(Slug, "invalid slug"))?;

        let (permapath, mut url): (Cow<'_, Path>, _) = match (kind, rendered) {
            (Kind::Index, true) => {
                let mut url = UrlBuf::from(group_perma);
                url.append("/");

                (group_perma.join("index.html").into(), url)
            }
            (Kind::Item(_), true) => {
                let dir = group_perma.join(&*slug);
                let mut url = UrlBuf::from(&*dir);
                url.append("/");

                (dir.join("index.html").into(), url)
            }
            (Kind::Datum(_), true) => return Ok(()),
            (_, false) => {
                let path = item.entry
                    .path_relative_to(content_root)
                    .unwrap();

                (path.into(), UrlBuf::from(&*path))
            },
        };

        url.make_relative().prepend(&self.config.settings.root);
        item.metadata.insert(PermaPath, permapath);
        item.metadata.insert(UrlRef, url);

        let template_name = match kind {
            Kind::Index => "index.html",
            Kind::Item(_) => "page.html",
            Kind::Datum(_) => "data.html",
        };

        let template = self.template_root.and_then(|subtree| {
            for parent in group_perma.ancestors() {
                let template_path = parent.join(template_name);
                if self.tree.get_file_id(subtree, &template_path).is_some() {
                    return Some(template_path);
                }

                let template_path = parent.with_extension("html");
                if self.tree.get_file_id(subtree, &template_path).is_some() {
                    return Some(template_path);
                }
            }

            self.tree.get_file_id(subtree, "default.html")
                .map(|_| PathBuf::from("default.html"))
        });

        if let Some(template_path) = template {
            item.metadata.insert(Template, template_path.to_path_buf().into_path_str_lossy());
        }

        Ok(())
    }

    fn render_site_item(&self, item: &Item) -> Result<()> {
        // TODO: Add cache key `?HASH`?
        let entry = &*item.entry;
        let permapath = match item.metadata.get(PermaPath) {
            Some(perma) => perma.map_err(|v| v.type_err(PermaPath, entry.path.display()))?,
            None => return Ok(()),
        };

        // TODO: Case-inensitive check.
        let output = self.output.join(&permapath);
        std::fs::create_dir_all(output.parent().unwrap())?;
        match entry.file_ext() {
            Some("scss") | Some("sass") => {
                Grass::default().map_copy(&*entry.path, output.with_extension("css"))
            },
            _ => entry.path.read_to(&output).chain_with(|| error! {
                "failed to copy asset",
                "source path" => entry.path.display(),
                "destination path" => output.display(),
            })
        }
    }
}
