use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use crate::markdown::Plugin;
use crate::taxonomy::Metadata;
use crate::templating::Engine;
use crate::error::{Result, Chainable};

pub struct Templatize<'m> {
    path: &'m Path,
    engine: Arc<dyn Engine>,
    metadata: &'m Metadata,
}

impl<'m> Templatize<'m>{
    pub fn with(path: &'m Path, engine: Arc<dyn Engine>, metadata: &'m Metadata) -> Self {
        Self { path, engine, metadata }
    }
}

impl Plugin for Templatize<'_> {
    fn preprocess<'a>(&self, input: &'a str) -> Result<Cow<'a, str>> {
        if !crate::util::is_template(input) {
            return Ok(Cow::Borrowed(input));
        }

        self.engine.render_str(self.path.to_str(), input, self.metadata.clone())
            .chain(error!("markdown templatization failed"))
            .map(Cow::Owned)
    }
}
