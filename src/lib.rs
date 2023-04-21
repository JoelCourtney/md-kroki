mod render;
#[cfg(test)]
mod test;

use anyhow::Result;
use std::path::Path;

pub struct MdKroki {
    endpoint: String,
    custom_tag: String,
    path_resolver: PathResolver,
}

#[allow(clippy::type_complexity)]
pub enum PathResolver {
    None,
    Path(Box<dyn Fn(&Path) -> Result<String> + Send>),
    PathAndRoot(Box<dyn Fn(&Path, Option<&str>) -> Result<String> + Send>),
}

impl MdKroki {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }

    pub fn custom_tag(mut self, custom_tag: String) -> Self {
        self.custom_tag = custom_tag;
        self
    }

    pub fn path_resolver(mut self, path_resolver: PathResolver) -> Self {
        self.path_resolver = path_resolver;
        self
    }
}

impl Default for MdKroki {
    fn default() -> Self {
        MdKroki {
            endpoint: "https://kroki.io".to_string(),
            custom_tag: "kroki".to_string(),
            path_resolver: PathResolver::None,
        }
    }
}
