//! [![Crates.io](https://img.shields.io/crates/v/md-kroki.svg)](https://crates.io/crates/md-kroki)
//! [![Docs.rs](https://docs.rs/md-kroki/badge.svg)](https://docs.rs/md-kroki)
//! [![CI](https://github.com/JoelCourtney/md-kroki/workflows/CI/badge.svg)](https://github.com/JoelCourtney/md-kroki/actions)
//!
//! This crate provides a tool for rendering [Kroki](https://kroki.io) diagrams inside markdown strings.
//! The input diagram code can either be inlined in the markdown or referenced via and external file, but
//! for now the output is always inlined back into the markdown.
//!
//! # Usage
//!
//! ## Creating a renderer
//!
//! You can create a default renderer easily:
//!
//! ```rust
//! # use md_kroki::MdKroki;
//! # tokio_test::block_on(async {
//! # let my_markdown_string: String = String::new();
//! // This default renderer uses the kroki.io API and only allows inlined diagrams.
//! let renderer = MdKroki::new();
//!
//! renderer.render(my_markdown_string).await
//! # });
//! ```
//!
//! The renderer also provides a synchronous `render_sync` method for sync contexts.
//!
//! You can configure the endpoint and enable external file references with the builder:
//!
//! ```rust
//! # use md_kroki::MdKroki;
//! # tokio_test::block_on(async {
//! # let my_markdown_string: String = String::new();
//! let renderer = MdKroki::builder()
//!
//!    // Use your own deployment of Kroki.
//!    .endpoint("http://localhost/")
//!
//!    // Resolve file references and read their contents.
//!    // See builder docs for more details.
//!    .path_resolver(|path| Ok(std::fs::read_to_string(path)?))
//!
//!    .build();
//!
//! renderer.render(my_markdown_string).await
//! # });
//! ```
//!
//! ## Inlining diagrams
//!
//! You can write the diagram code directly in the markdown using the custom `<kroki>` tag:
//!
//! ```md
//! <kroki type="erd">
//!   [Person]
//!   *name
//!   height
//!   weight
//!   +birth_location_id
//!
//!   [Location]
//!   *id
//!   city
//!   state
//!   country
//!
//!   Person *--1 Location
//! </kroki>
//! ```
//!
//! The `type` attribute tells kroki what renderer to use and is required.
//!
//! If you want to use traditional markdown elements, you can inline the diagram source with a fenced code block.
//!
//! ``````markdown
//! ```kroki-mermaid
//! graph TD
//!   A[ Anyone ] -->|Can help | B( Go to github.com/yuzutech/kroki )
//!   B --> C{ How to contribute? }
//!   C --> D[ Reporting bugs ]
//!   C --> E[ Sharing ideas ]
//!   C --> F[ Advocating ]
//! ```
//! ``````
//!
//! Here the code block language takes the place of the `type` attribute: it must be of the form `kroki-<diagram type>`.
//! Otherwise it will be treated like a normal code block.
//!
//! ## Referencing external files
//!
//! If the input code of a diagram is too big to inline nicely in your markdown, you can reference an external file:
//!
//! ```md
//! Using the kroki tag:
//! <kroki type="excalidraw" path="my/file.excalidraw" />
//!
//! Or using a traditional markdown image tag:
//! ![my excalidrawing](kroki-excalidraw:my/file.excalidraw)
//! ```
//!
//! When using the markdown tag, the path must be prefixed with `kroki-<diagram type>:`. Otherwise it will be treated
//! like a normal image tag.
//!
//! You must provide a path resolver to the builder if you want to use file references.

#![deny(missing_docs)]

mod render;
#[cfg(test)]
mod test;

use anyhow::Result;
use std::path::PathBuf;

/// Kroki diagram renderer.
pub struct MdKroki {
    endpoint: String,
    path_resolver: PathResolver,
}

impl MdKroki {
    /// Create a default renderer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder.
    pub fn builder() -> MdKrokiBuilder {
        MdKrokiBuilder::new()
    }
}

/// Options for resolving paths in tags that reference external files.
///
/// It will cause an error if you use a path without providing an appropriate resolver.
#[allow(clippy::type_complexity)]
#[derive(Default)]
enum PathResolver {
    #[default]
    None,
    Path(Box<dyn Fn(PathBuf) -> Result<String> + Send>),
    PathAndRoot(Box<dyn Fn(PathBuf, Option<&str>) -> Result<String> + Send>),
}

/// Builder for configuring the renderer.
pub struct MdKrokiBuilder {
    endpoint: String,
    path_resolver: PathResolver,
}

impl MdKrokiBuilder {
    /// Creates a builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the endpoint url. Use if you'd like to target your own deployment of Kroki.
    ///
    /// Default is <https://kroki.io>.
    pub fn endpoint(mut self, endpoint: impl std::fmt::Display) -> Self {
        self.endpoint = endpoint.to_string();
        self
    }

    /// Sets a basic path resolver. Unnecessary if all your diagrams are inline. Example:
    ///
    /// ```
    /// # use std::path::Path;
    /// # use md_kroki::MdKroki;
    /// let resolver = |path| {
    ///     let base_path = Path::new("path/to/files");
    ///     Ok(std::fs::read_to_string(base_path.join(path))?)
    /// };
    /// let md_kroki = MdKroki::builder()
    ///     .path_resolver(resolver)
    ///     .build();
    /// ```
    pub fn path_resolver<F>(mut self, path_resolver: F) -> Self
    where
        F: Fn(PathBuf) -> Result<String> + Send + 'static,
    {
        self.path_resolver = PathResolver::Path(Box::new(path_resolver));
        self
    }

    /// Path resolver with optional root parameter.
    ///
    /// If none of your diagrams use a root attribute, just use [path_resolver][Self::path_resolver].
    /// There is no need to provide both [path_resolver][Self::path_resolver] and [path_and_root_resolver][Self::path_and_root_resolver].
    ///
    /// This option is only available on external file references on the
    /// `<kroki>` tag. Using the `root` attribute will send that value to the resolver:
    ///
    /// ```xml
    /// <kroki type="mermaid" path="file.mermaid" root="assets" />
    /// ```
    ///
    /// In most cases this option will be unnecessary. Example:
    ///
    /// ```
    /// # use std::path::Path;
    /// # use md_kroki::MdKroki;
    /// # use anyhow::bail;
    /// let resolver = |path, root: Option<&str>| {
    ///     let base_path = match root {
    ///         None => Path::new(""),
    ///         Some("assets") => Path::new("static/assets"),
    ///         Some(r) => bail!("unrecognized root: {r}")
    ///     };
    ///     Ok(std::fs::read_to_string(base_path.join(path))?)
    /// };
    /// let md_kroki = MdKroki::builder()
    ///     .path_and_root_resolver(resolver)
    ///     .build();
    /// ```
    ///
    /// Due to limitations in Rust's type inference, you need to specify `Option<&str>` as the
    /// type of the `root` argument. It can't be inferred.
    pub fn path_and_root_resolver<F>(mut self, path_resolver: F) -> Self
    where
        F: Fn(PathBuf, Option<&str>) -> Result<String> + Send + 'static,
    {
        let wrapped = move |path, root: Option<&str>| path_resolver(path, root);
        self.path_resolver = PathResolver::PathAndRoot(Box::new(wrapped));
        self
    }

    /// Consume self and build a renderer.
    pub fn build(self) -> MdKroki {
        MdKroki {
            endpoint: self.endpoint,
            path_resolver: self.path_resolver,
        }
    }
}

impl Default for MdKroki {
    fn default() -> Self {
        MdKroki {
            endpoint: "https://kroki.io".to_string(),
            path_resolver: PathResolver::None,
        }
    }
}

impl Default for MdKrokiBuilder {
    fn default() -> Self {
        MdKrokiBuilder {
            endpoint: "https://kroki.io".to_string(),
            path_resolver: PathResolver::None,
        }
    }
}
