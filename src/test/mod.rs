use crate::MdKroki;
use crate::PathResolver;
use pretty_assertions::assert_eq;
use std::path::Path;
use tokio::test;

macro_rules! test_from_files {
    ($dir:expr, $renderer:expr) => {
        let input = std::fs::read_to_string(format!("src/test/{}/input.md", $dir)).unwrap();
        let output = std::fs::read_to_string(format!("src/test/{}/output.md", $dir)).unwrap();
        assert_eq!(
            $renderer.render_async(input.clone()).await.unwrap(),
            output.clone()
        );
        tokio::task::spawn_blocking(move || assert_eq!($renderer.render(input).unwrap(), output))
            .await
            .unwrap();
    };
}

macro_rules! path_resolver {
    ($dir:expr) => {{
        let resolver = |path: &Path| {
            let base_path_string = format!("src/test/{}", $dir);
            let base_path = Path::new(&base_path_string);
            Ok(std::fs::read_to_string(base_path.join(path))?)
        };
        PathResolver::Path(Box::new(resolver))
    }};
}

#[test]
async fn inline_md() {
    test_from_files!("inline_md", MdKroki::new());
}

#[test]
async fn inline_xml() {
    test_from_files!("inline_xml", MdKroki::new());
}

#[test]
async fn reference_md() {
    const DIR: &str = "reference_md";
    test_from_files!(DIR, MdKroki::new().path_resolver(path_resolver!(DIR)));
}

#[test]
async fn reference_xml() {
    const DIR: &str = "reference_xml";
    test_from_files!(DIR, MdKroki::new().path_resolver(path_resolver!(DIR)));
}
