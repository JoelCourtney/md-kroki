use crate::MdKroki;
use pretty_assertions::assert_eq;
use std::path::{Path, PathBuf};
use tokio::test;

async fn test_from_files(dir: &str, renderer: MdKroki) {
    let input = std::fs::read_to_string(format!("src/test/{dir}/input.md")).unwrap();
    let output = std::fs::read_to_string(format!("src/test/{dir}/output.md")).unwrap();
    assert_eq!(
        renderer.render(input.clone()).await.unwrap(),
        output.clone()
    );
    tokio::task::spawn_blocking(move || assert_eq!(renderer.render_sync(input).unwrap(), output))
        .await
        .unwrap();
}

fn path_resolver(dir: &'static str) -> impl Fn(PathBuf) -> anyhow::Result<String> {
    move |path| {
        let base_path_string = format!("src/test/{}", dir);
        let base_path = Path::new(&base_path_string);
        Ok(std::fs::read_to_string(base_path.join(path))?)
    }
}

#[test]
async fn inline_md() {
    test_from_files("inline_md", MdKroki::new()).await;
}

#[test]
async fn inline_xml() {
    test_from_files("inline_xml", MdKroki::new()).await;
}

#[test]
async fn reference_md() {
    const DIR: &str = "reference_md";
    test_from_files(
        DIR,
        MdKroki::builder()
            .path_resolver(path_resolver(DIR))
            .build(),
    )
    .await;
}

#[test]
async fn reference_xml() {
    const DIR: &str = "reference_xml";
    test_from_files(
        DIR,
        MdKroki::builder()
            .path_resolver(path_resolver(DIR))
            .build(),
    )
    .await;
}

#[test]
async fn embedded_in_document() {
    const DIR: &str = "embedded_in_document";
    test_from_files(
        DIR,
        MdKroki::builder()
            .path_resolver(path_resolver(DIR))
            .build(),
    )
    .await;
}
