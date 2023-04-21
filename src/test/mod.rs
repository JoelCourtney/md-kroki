use crate::MdKroki;
use pretty_assertions::assert_eq;
use tokio::test;

macro_rules! test_from_files {
    ($dir:literal, $renderer:expr) => {
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

#[test]
async fn basic() {
    test_from_files!("basic", MdKroki::default());
}
