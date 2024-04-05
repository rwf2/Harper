use std::sync::Arc;

use rayon::prelude::*;

use crate::error::Result;
use crate::taxonomy::*;

#[inline(always)]
pub fn render_site<R>(renderer: &R, site: &Arc<Site>) -> Result<R::Output>
    where R: Renderer + ?Sized
{
    let (collected, process_result): (Result<R::Output>, _) = rayon::join(
        || site.collections.par_iter()
            .map(|(_, collection)| renderer.render_collection(site, collection))
            .collect(),
        || site.items.par_iter().try_for_each(|asset| renderer.render_site_item(asset))
    );

    match (collected, process_result) {
        (Ok(v), Ok(_)) => Ok(v),
        (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
        (Err(e1), Err(e2)) => Err(e1.chain(e2)),
    }
}

#[inline(always)]
pub fn render_collection<R>(
    renderer: &R,
    site: &Arc<Site>,
    collection: &Arc<Collection>,
) -> Result<R::Collected>
    where R: Renderer + ?Sized
{
    rayon::join(
        || collection.items.sort_by(|a, b| a.entry.path.cmp(&b.entry.path)),
        || collection.data.par_iter().for_each(|(_, l)| {
            l.sort_by(|a, b| a.entry.path.cmp(&b.entry.path))
        }),
    );

    collection.par_map_items(|kind, item| {
        renderer.render_collection_item(kind, site, collection, item)
    })
}

pub trait Renderer: Sync {
    type Output: FromParallelIterator<Self::Collected> + Send;

    type Collected: FromParallelIterator<Self::Render> + Send;

    type Render: Send;

    #[inline(always)]
    fn render_site(&self, site: &Arc<Site>) -> Result<Self::Output> {
        render_site(self, site)
    }

    #[inline(always)]
    fn render_collection(
        &self,
        site: &Arc<Site>,
        collection: &Arc<Collection>
    ) -> Result<Self::Collected> {
        render_collection(self, site, collection)
    }

    fn render_collection_item(&self,
        kind: Kind,
        site: &Arc<Site>,
        collection: &Arc<Collection>,
        item: &Arc<Item>
    ) -> Result<Self::Render>;

    fn render_site_item(&self, item: &Item) -> Result<()>;
}
