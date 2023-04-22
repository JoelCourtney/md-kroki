# md-kroki

[![Crates.io](https://img.shields.io/crates/v/md-kroki.svg)](https://crates.io/crates/md-kroki)
[![Docs.rs](https://docs.rs/md-kroki/badge.svg)](https://docs.rs/md-kroki)
[![CI](https://github.com/JoelCourtney/md-kroki/workflows/CI/badge.svg)](https://github.com/JoelCourtney/md-kroki/actions)

This crate provides a tool for rendering [Kroki](https://kroki.io) diagrams inside markdown strings.
The input diagram code can either be inlined in the markdown or referenced via and external file, but
for now the output is always inlined back into the markdown.

## Usage

### Creating a renderer

You can create a default renderer easily:

```rust
# use md_kroki::MdKroki;
# tokio_test::block_on(async {
# let my_markdown_string: String = String::new();
// This default renderer uses the kroki.io API and only allows inlined diagrams.
let renderer = MdKroki::new();

renderer.render(my_markdown_string).await
# });
```

The renderer also provides a synchronous `render_sync` method for sync contexts.

You can configure the endpoint and enable external file references with the builder:

```rust
# use md_kroki::MdKroki;
# tokio_test::block_on(async {
# let my_markdown_string: String = String::new();
let renderer = MdKroki::builder()

   // Use your own deployment of Kroki.
   .endpoint("http://localhost/")

   // Resolve file references and read their contents.
   // See builder docs for more details.
   .path_resolver(|path| Ok(std::fs::read_to_string(path)?))

   .build();

renderer.render(my_markdown_string).await
# });
```

### Inlining diagrams

You can write the diagram code directly in the markdown using the custom `<kroki>` tag:

```md
<kroki type="erd">
  [Person]
  *name
  height
  weight
  +birth_location_id

  [Location]
  *id
  city
  state
  country

  Person *--1 Location 
</kroki>
```

The `type` attribute tells kroki what renderer to use and is required.

If you want to use traditional markdown elements, you can inline the diagram source with a fenced code block.

``````markdown
```kroki-mermaid
graph TD
  A[ Anyone ] -->|Can help | B( Go to github.com/yuzutech/kroki )
  B --> C{ How to contribute? }
  C --> D[ Reporting bugs ]
  C --> E[ Sharing ideas ]
  C --> F[ Advocating ]
```
``````

Here the code block language takes the place of the `type` attribute: it must be of the form `kroki-<diagram type>`.
Otherwise it will be treated like a normal code block.

### Referencing external files

If the input code of a diagram is too big to inline nicely in your markdown, you can reference an external file:

```md
Using the kroki tag:
<kroki type="excalidraw" path="my/file.excalidraw" />

Or using a traditional markdown image tag:
![my excalidrawing](kroki-excalidraw:my/file.excalidraw)
```

When using the markdown tag, the path must be prefixed with `kroki-<diagram type>:`. Otherwise it will be treated
like a normal image tag.

You must provide a path resolver to the builder if you want to use file references.

## License

Licensed under: Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
