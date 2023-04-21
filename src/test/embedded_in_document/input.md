# Embedding all types in a longer document

Inline markdown code block:

```kroki-seqdiag
seqdiag {
  browser  -> webserver [label = "GET /seqdiag/svg/base64"];
  webserver  -> processor [label = "Convert text to image"];
  webserver <-- processor;
  browser <-- webserver;
}
```

Inline kroki tag:

<kroki type="dbml">
Table users {
  id integer
  username varchar
  role varchar
  created_at timestamp
}

Table posts {
  id integer [primary key]
  title varchar
  body text [note: 'Content of the post']
  user_id integer
  status post_status
  created_at timestamp
}

Enum post_status {
  draft
  published
  private [note: 'visible via URL only']
}

Ref: posts.user_id > users.id // many-to-one
</kroki>

Reference markdown image tag:

![Excalidraw](kroki-excalidraw:reference_md.excalidraw)

Reference kroki tag:

<kroki type="pikchr" path="reference_xml.pikchr"/>

```rust
fn main() {
  println!("neat");
}
```
