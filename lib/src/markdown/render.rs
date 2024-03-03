use pulldown_cmark::{html, Event};

use crate::markdown::Plugin;
use crate::error::Result;
use crate::value::Sink;

#[derive(Clone)]
pub struct Renderer<O> {
    output: O,
    rendered: String,
}

impl<O: Sink> Renderer<O> {
    pub fn new(output: O) -> Self {
        Renderer { output, rendered: String::new() }
    }
}

impl<O: Sink> Plugin for Renderer<O> {
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>>
    {
        let mut html_output = String::new();
        html::push_html(&mut html_output, events);
        self.rendered = html_output;
        Box::new(std::iter::empty())
    }

    fn finalize(&mut self) -> Result<()> {
        let string = std::mem::replace(&mut self.rendered, String::new());
        self.output.write(string)
    }
}
