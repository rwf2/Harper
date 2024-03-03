use std::borrow::Cow;
use std::fmt::Write;

use crate::error::Result;

// TODO: Rename to "alert". Follow GitHub syntax?

#[derive(Default, Clone)]
pub struct Admonition;

impl crate::markdown::Plugin for Admonition {
    fn preprocess<'a>(&self, input: &'a str) -> Result<Cow<'a, str>> {
        let mut output = Cow::Borrowed(input);

        let mut bytes = input.as_bytes();
        while let Some(i) = memchr::memchr(b'!', &bytes) {
            let k = (input.len() - bytes.len()) + i;
            if is_line_start(k, input) {
                // Insert the admonition into the stream. If the output is still
                // borrowed, shorten the contents to the slice before the !.
                // Note that we're guaranteed to be at a line start.
                if matches!(output, Cow::Borrowed(_)) {
                    output = Cow::Owned(input[..k].into());
                } else if let Cow::Owned(buf) = &mut output {
                    let j = input.len() - bytes.len();
                    buf.push_str(&input[j..k]);
                }

                // Parse the admonition header and seek to the end of it.
                let (name, title, h_end) = parse_admonition_header(&input[(k + 1)..]);
                bytes = &bytes[(h_end + i + 1)..];

                // Find the end of the admonition and capture the whole thing.
                let end = find_admonition_end(bytes);
                let admonition = &bytes[..end];
                bytes = &bytes[end..];

                let buf = output.to_mut();
                let _ = writeln!(buf, r#"<div class="admonition {name}">"#);
                let _ = write!(buf, r#"<span class="title {name}">"#);
                let inner = std::str::from_utf8(admonition).unwrap();
                let _ = writeln!(buf, "\n\n{title}\n\n</span>\n\n{inner}\n</div>\n");
            } else {
                if let Cow::Owned(buf) = &mut output {
                    let j = input.len() - bytes.len();
                    buf.push_str(&input[j..(k + 1)]);
                }

                bytes = &bytes[(i + 1)..];
            }
        }

        if let Cow::Owned(buf) = &mut output {
            if !bytes.is_empty() {
                let j = input.len() - bytes.len();
                buf.push_str(&input[j..]);
            }
        }

        Ok(output)
    }
}

fn is_line_start(i: usize, string: &str) -> bool {
    i == 0 || string.as_bytes().get(i - 1).map_or(false, |s| *s == b'\n')
}

/// Returns `name`, `title`, index in `string` for end of header.
fn parse_admonition_header(string: &str) -> (&str, &str, usize) {
    let bytes = string.as_bytes();
    match memchr::memchr2(b':', b'\n', bytes) {
        Some(i) if bytes[i] == b':' => {
            let mut k = i + 1;
            let end = loop {
                if let Some(j) = memchr::memchr(b'\n', &bytes[k..]) {
                    k += j + 1;
                    if bytes[k..].starts_with(b"\n") {
                        break k;
                    }
                } else {
                    break string.len()
                }
            };

            let name = string[..i].trim();
            let title = string[i + 1..end].trim();
            (name, title, end)
        }
        Some(i) if bytes[i] == b'\n' => {
            let name = string[..i].trim();
            (name, &string[0..0], i + 1)
        }
        _ => {
            let name = string.trim();
            (name, &string[0..0], string.len())
        }
    }
}

/// Given the start of an admonition in `bytes`, returns the index in `bytes`
/// where the first non-admonition text can be found.
fn find_admonition_end(mut bytes: &[u8]) -> usize {
    let start = bytes;
    loop {
        if bytes.starts_with(b"\n") {
            bytes = &bytes[1..];
            continue;
        }

        if !bytes.starts_with(b"  ") {
            break;
        }

        match memchr::memchr(b'\n', bytes) {
            Some(i) => bytes = &bytes[(i + 1)..],
            None => bytes = &bytes[bytes.len()..],
        }
    }

    start.len() - bytes.len()
}
