use std::borrow::Cow;

use crate::error::Result;
use crate::value::{Mapper, Sink};

#[derive(Default, Clone)]
pub struct FrontMatter<M: Mapper, O: Sink> {
    mapper: M,
    output: O
}

impl<M: Mapper, O: Sink> FrontMatter<M, O> {
    pub fn new(mapper: M, output: O) -> Self { Self { mapper, output } }
}

impl<M: Mapper, O: Sink> crate::markdown::Plugin for FrontMatter<M, O> {
    fn preprocess<'a>(&self, input: &'a str) -> Result<Cow<'a, str>> {
        const PREFIX: &str = "+++\n";
        const SUFFIX: &str = "\n+++\n";

        if !input.starts_with(PREFIX) {
            return Ok(Cow::Borrowed(input));
        }

        let (front_matter, content) = match input.split_once(SUFFIX) {
            Some((prefix, content)) => (&prefix[PREFIX.len()..], content),
            None => return Ok(Cow::Borrowed(input))
        };

        self.mapper.try_map_copy(front_matter, &self.output)?;
        Ok(Cow::Borrowed(content))
    }
}
