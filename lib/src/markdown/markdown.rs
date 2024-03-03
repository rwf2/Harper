use std::borrow::Cow;
use std::sync::Arc;

use pulldown_cmark::{Parser, Options};

use crate::error;
use crate::util::hlist::{HList, for_each_mut};
use crate::markdown::Plugin;
use crate::util::hlist::*;
use crate::error::{Chainable, Result};
use crate::value::Source;

#[derive(Debug, Clone)]
pub struct Markdown<I, P = Nil> {
    input: I,
    options: Options,
    plugins: P,
}

impl<I: Source> Markdown<I, Nil> {
    pub fn from(input: I) -> Self {
        Self {
            input,
            options: Options::all().difference(Options::ENABLE_SMART_PUNCTUATION),
            plugins: Nil,
        }
    }
}

impl<'a, I, P: HList> Markdown<I, P> {
    pub fn plugin<T: Plugin>(self, plugin: T) -> Markdown<I, HList![T, ..P]> {
        Markdown {
            input: self.input,
            options: self.options,
            plugins: self.plugins.insert(plugin)
        }
    }

    pub fn with_options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }
}

macro_rules! impl_generic {
    (@[$($T:ident)*]) => (
        impl<In: Source, $($T: Plugin),*> Markdown<In, HList![$($T),*]> {
            #[allow(unused_mut)]
            pub fn run(mut self) -> Result<Markdown<String, Nil>> {
                // println!("plugins: {}", stringify!($($T),*));
                // println!("  ++> {}", std::any::type_name::<Self>());
                // $(println!("  --> {} = {}", stringify!($T), std::any::type_name::<$T>());)*

                let input = self.input.try_read::<Arc<str>>()?;
                let input = Cow::Owned(input.to_string());
                let input = rfold!([$($T)*] self.plugins.to_ref(), input,
                    |p, input| {
                        match input {
                            Cow::Borrowed(input) => p.preprocess(input)?,
                            Cow::Owned(input) => {
                                let i = input.as_str();
                                match p.preprocess(i)? {
                                    Cow::Borrowed(s) if s.as_ptr() == i.as_ptr() => Cow::Owned(input),
                                    Cow::Borrowed(s) => Cow::Owned(s.to_string()),
                                    Cow::Owned(s) => Cow::Owned(s)
                                }
                            }
                        }
                    }
                );

                let parser = Parser::new_ext(&input, self.options);
                let events = rfold!([$($T)*] self.plugins.to_mut(), parser,
                    |p, events| p.remap(events)
                );

                // Run the iterator.
                events.for_each(|_| {});

                let string: String = input.into_owned();
                for_each_mut!(
                    [$($T)*] self.plugins.to_mut(),
                    |p| p.finalize().chain(error!("markdown plugin failed"))?
                );

                Ok(Markdown::from(string).with_options(self.options))
            }
        }
    );

    ([]) => (impl_generic!(@[]););
    ([$T:ident $($R:ident)*]) => (
        impl_generic!(@[$T $($R)*]);
        impl_generic!([$($R)*]);
    );
}

impl_generic!([A B C D E F G H I J K L M N O P Q R S T U V W X Y Z]);
