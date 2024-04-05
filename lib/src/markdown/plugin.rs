use std::borrow::Cow;

use pulldown_cmark::Event;

use crate::error::Result;

pub trait Plugin {
    #[inline(always)]
    fn preprocess<'a>(&self, input: &'a str) -> Result<Cow<'a, str>> {
        Ok(Cow::Borrowed(input))
    }

    #[inline(always)]
    fn remap<'a, I>(&'a mut self, events: I) -> impl Iterator<Item = Event<'a>> + 'a
        where I: Iterator<Item = Event<'a>> + 'a
    {
        events
    }

    #[inline(always)]
    fn finalize(&mut self) -> Result<()> {
        Ok(())
    }
}
