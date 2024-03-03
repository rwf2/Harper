#![doc = svgbobdoc::transform!(
//! A toolkit for creating blazing fast static site generators.
//!
//! # Overview
//!
//! Sketch is a library for creating _really fast_ static site generators:
//! programs that consume files and emit static HTML web sites and their assets.
//! It does not enjoin any particular structure on source content nor operations
//! on said content.
//!
//! Internally, harper organizes content in a semi-hierarchy as follows:
//!
//! ```svgbob
//!                            +-------+
//!                            | Site  |
//!                            +---+---+
//!                                |
//!  +-----------------------------+--------------------------------+
//!  |                                                              |
//!  |  +------------+    +------------+    +-------+    +-------+  |
//!  |  | Collection |... | Collection |    | Asset |... | Asset |  |
//!  |  +------------+    +------+-----+    +-------+    +-------+  |
//!  |                           |                                  |
//!  |    +----------------------+----------------------+           |
//!  |    |      +--------+           +------------+    |           |
//!  |    |      | index? |           | data group |    |           |
//!  |    |      +--------+           +-----+------+    |           |
//!  |    |  +------+   +------+   +------+ | +------+  |           |
//!  |    |  | item |...| item |   | data |...| data |  |           |
//!  |    |  +------+   +------+   +------+   +------+  |           |
//!  |    +---------------------------------------------+           |
//!  +--------------------------------------------------------------+
//! ```
//!
//! In words, a **site** consists of:
//!
//!   * **Collections**, consisting of:
//!
//!     - _Items_, potentially sorted, one of which may be the _index_. Items
//!     are represented as a dictionary of string keys and JSON-like values.
//!     This dictionary is called the item's _metadata_. All item data is stored
//!     in its _metadata_. Metadata is transformed as item processing proceeds.
//!
//!     - _Data_ organized into _data groups_.
//!
//!     You can think of a collection as a directory and its contents
//!     partitioned into text content (_items_) and data, though it need not
//!     necessarily be organized in this fashion.
//!
//!   * **Assets**
//!
//!     Assets, such as images and stylesheets, which are rendered according to
//!     a pipeline corresponding to the file's type.
//!
//! ## Rendering
//!
//! A site is typically rendered via the following set of operations:
//!
//! 1. Files in a directory are read into collections.
//! 2. Items' metadata is transformed according to the SSG's liking. A typical
//!    transformation might look like:
//!    - Preamble metadata is read directly into the file's metadata.
//!    - Item text is rendered as markdown into HTML and stored in a `content`
//!      metadata field.
//!    - The item is rendered into a template using the item's `metadata` as the
//!      templating context.
//! 3. Assets are transformed according to the SSG's liking.
//! 4. Fully transformed content is written to an output directory.
)]

#[macro_use]
pub mod error;
pub mod util;
pub mod fstree;
pub mod value;
pub mod taxonomy;
pub mod markdown;
pub mod templating;
pub mod path_str;
pub mod url;

pub use taxonomy::*;

pub use rayon;
