use crate::{MdKroki, PathResolver};
use anyhow::anyhow;
use anyhow::{bail, Result};
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag};
use serde::Serialize;
use sscanf::sscanf;
use std::ops::Range;
use std::path::PathBuf;
use xmltree::Element;

impl MdKroki {
    /// Asynchronously render and inline diagrams into the provided markdown string.
    ///
    /// Diagram render requests are awaited in parallel.
    pub async fn render(&self, mut content: String) -> Result<String> {
        let client = reqwest::Client::new();

        let renders = self.get_render_requests(&content)?;

        let replace_futures = renders.map(|render| async {
            let result = client
                .post(&self.endpoint)
                .body(serde_json::to_string(&render).expect("could no serialize kroki request"))
                .send()
                .await
                .expect("could not send kroki request")
                .error_for_status()?
                .text()
                .await?;
            let result = process_xml(result)?;
            Ok::<ReplaceRequest, anyhow::Error>(ReplaceRequest {
                range: render.replace_range,
                content: result,
            })
        });

        let mut replaces = futures::future::join_all(replace_futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        replaces.sort_by_key(|r| r.range.start);

        for replace in replaces.into_iter().rev() {
            let trimmed_range = trim_replace_range(&content, &replace.range);
            content.replace_range(trimmed_range, &replace.content)
        }

        Ok(content)
    }

    /// Synchronously render and inline diagrams into the provided markdown string.
    ///
    /// Should only be called from a sync context. In an async context, the normal [render][MdKroki::render] method
    /// is recommended.
    pub fn render_sync(&self, mut content: String) -> Result<String> {
        let client = reqwest::blocking::Client::new();

        let renders = self.get_render_requests(&content)?;

        let mut replaces = renders
            .map(|req| {
                let result = client
                    .post(&self.endpoint)
                    .body(serde_json::to_string(&req).expect("could no serialize kroki request"))
                    .send()
                    .expect("could not send kroki request")
                    .error_for_status()?
                    .text()?;
                let result = process_xml(result)?;
                Ok::<ReplaceRequest, anyhow::Error>(ReplaceRequest {
                    range: req.replace_range,
                    content: result,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        replaces.sort_by_key(|r| r.range.start);

        for replace in replaces.into_iter().rev() {
            let trimmed_range = trim_replace_range(&content, &replace.range);
            content.replace_range(trimmed_range, &replace.content)
        }

        Ok(content)
    }

    fn get_render_requests<'a>(
        &self,
        content: &str,
    ) -> Result<impl Iterator<Item = RenderRequest> + 'a> {
        #[derive(PartialEq, Eq)]
        enum ParserState {
            InImage {
                diagram_type: String,
                diagram_source: String,
                replace_start: usize,
            },
            InKrokiReferenceTag {
                diagram_type: String,
                diagram_source: String,
                replace_start: usize,
            },
            InKrokiInlineTag {
                diagram_type: String,
                content_start: usize,
                replace_start: usize,
            },
            InCode {
                diagram_type: String,
            },
            InPre(usize),
            Out,
        }

        let mut state = ParserState::Out;

        let mut requests = Vec::new();

        Parser::new_ext(content, Options::all()).into_offset_iter().try_for_each(|(e, offset)| {
                match e {
                    Event::Html(ref tag) if tag.as_ref() == "<pre>" => {
                        state = match state {
                            ParserState::InPre(n) => ParserState::InPre(n+1),
                            _ => ParserState::InPre(1)
                        };
                    }
                    Event::Html(ref tag) if tag.as_ref() == "</pre>" => {
                        match &state {
                            ParserState::InPre(n@2..) => { state = ParserState::InPre(n-1) }
                            ParserState::InPre(1) => { state = ParserState::Out }
                            _ => {}
                        };
                    }
                    _ if matches!(state, ParserState::InPre(_)) => {}
                    Event::Html(ref tag) if tag.as_ref().starts_with("<kroki") => {
                        let (xml, closed) = if !tag.contains("/>") && !tag.contains("</kroki>") {
                            (tag.to_string() + "</kroki>", false)
                        } else {
                            (tag.to_string(), true)
                        };
                        let element = Element::parse(xml.as_bytes())?;
                        let diagram_type = element.attributes.get("type").ok_or_else(|| anyhow!("missing type tag"))?.clone();
                        if !element.attributes.contains_key("path") {
                            if closed {
                                bail!("kroki tag must either have an inlined diagram or a `path` attribute.");
                            }
                            state = ParserState::InKrokiInlineTag { diagram_type, content_start: offset.end, replace_start: offset.start };
                            return Ok(());
                        }
                        let path: PathBuf = element.attributes.get("path")
                            .ok_or_else(|| anyhow!("src tag required"))?.parse()?;
                        let path_root = element.attributes.get("root").map(|s| s.as_str());
                        let diagram_source = match &self.path_resolver {
                            PathResolver::None => bail!("path resolver required for content with file references"),
                            PathResolver::Path(res) => {
                                if path_root.is_some() {
                                    bail!("path resolver must accept a root argument for content that uses it");
                                }
                                res(path)?
                            }
                            PathResolver::PathAndRoot(res) => res(path, path_root)?
                        };
                        if closed {
                            requests.push(RenderRequest {
                                diagram_source,
                                diagram_type,
                                output_format: "svg".to_string(),
                                replace_range: offset
                            })
                        } else {
                            state = ParserState::InKrokiReferenceTag { diagram_type, diagram_source, replace_start: offset.start }
                        }
                    }
                    Event::Html(ref tag) if tag.contains("</kroki>") => {
                        if let ParserState::InKrokiInlineTag { ref diagram_type, content_start, replace_start } = state {
                            let diagram_source = content[content_start..offset.start].to_string();
                            requests.push(RenderRequest {
                                diagram_source,
                                diagram_type: diagram_type.clone(),
                                output_format: "svg".to_string(),
                                replace_range: replace_start .. offset.end
                            });
                            state = ParserState::Out;
                        } else if let ParserState::InKrokiReferenceTag { ref diagram_type, ref diagram_source, replace_start } = state {
                            requests.push(RenderRequest {
                                diagram_source: diagram_source.clone(),
                                diagram_type: diagram_type.clone(),
                                output_format: "svg".to_string(),
                                replace_range: replace_start .. offset.end
                            });
                            state = ParserState::Out;
                        }
                    }
                    _ if matches!(state, ParserState::InKrokiReferenceTag {..} | ParserState::InKrokiInlineTag {..}) => {},
                    Event::Start(Tag::Image(LinkType::Inline, ref url, _)) => {
                        if let Ok((diagram_type, path)) = sscanf!(url, "kroki-{String}:{PathBuf}") {
                            let diagram_source = match &self.path_resolver {
                                PathResolver::None => bail!("path resolver required for content with file references"),
                                PathResolver::Path(res) => res(path)?,
                                PathResolver::PathAndRoot(res) => res(path, None)?
                            };
                            state = ParserState::InImage { diagram_type, diagram_source, replace_start: offset.start };
                        }
                    }
                    Event::End(Tag::Image(..)) => {
                        if let ParserState::InImage { ref diagram_type, ref diagram_source, replace_start } = state {
                            requests.push(RenderRequest {
                                diagram_source: diagram_source.to_string(),
                                diagram_type: diagram_type.clone(),
                                output_format: "svg".to_string(),
                                replace_range: replace_start .. offset.end
                            });
                            state = ParserState::Out;
                        }
                    }
                    Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                        if let Ok(diagram_type) = sscanf!(lang, "kroki-{String}") {
                            state = ParserState::InCode { diagram_type }
                        }
                    }
                    Event::End(Tag::CodeBlock(..)) => {
                        if let ParserState::InCode { ref diagram_type } = state {
                            let content_start = content[offset.clone()].trim_start().find(char::is_whitespace).ok_or_else(|| anyhow!("code block needs whitespace somewhere"))? + offset.start;
                            let content_end = content[offset.clone()].trim_end().rfind(|c| c != '`').unwrap() + offset.start + 1;
                            let diagram_source = content[content_start..content_end].to_string();
                            requests.push(RenderRequest {
                                diagram_source,
                                diagram_type: diagram_type.clone(),
                                output_format: "svg".to_string(),
                                replace_range: offset
                            });
                            state = ParserState::Out;
                        }
                    }
                    _ => {},
                }
                Ok(())
            })?;

        Ok(requests.into_iter())
    }
}

#[derive(Serialize, Debug)]
struct RenderRequest {
    diagram_source: String,
    diagram_type: String,
    output_format: String,

    #[serde(skip)]
    replace_range: Range<usize>,
}

struct ReplaceRequest {
    range: Range<usize>,
    content: String,
}

fn trim_replace_range(content: &str, range: &Range<usize>) -> Range<usize> {
    let new_start =
        range.start + (range.len() - content[range.start..range.end].trim_start().len());
    let new_end = range.end - (range.len() - content[range.start..range.end].trim_end().len());
    new_start..new_end
}

fn process_xml(mut xml: String) -> Result<String> {
    let start_index = xml
        .find("<svg")
        .unwrap_or_else(|| panic!("didn't find '<svg' in kroki response: {}", xml));
    xml.replace_range(..start_index, "");
    xml.insert_str(0, "<pre>");
    let end_index = xml
        .rfind("</svg>")
        .unwrap_or_else(|| panic!("didn't find '</svg>' in kroki response: {}", xml));
    xml.insert_str(end_index + 6, "</pre>");
    Ok(xml.trim().to_string())
}
