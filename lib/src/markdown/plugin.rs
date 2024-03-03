use std::borrow::Cow;

use pulldown_cmark::Event;

use crate::error::Result;

pub trait Plugin {
    #[inline(always)]
    fn preprocess<'a>(&self, input: &'a str) -> Result<Cow<'a, str>> {
        Ok(Cow::Borrowed(input))
    }

    // FIXME: To get rid of this box, we need (ideally) `-> impl Trait` in trait
    // methods, or generic associated types. Edit: We now have GATs! But they're
    // incredibly annoying to use because of the required bounds everywhere.
    #[inline(always)]
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(events)
    }

    #[inline(always)]
    fn finalize(&mut self) -> Result<()> {
        Ok(())
    }
}
