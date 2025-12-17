use std::path::Path;

use super::*;

struct TestLoader;

impl ConfigLoader for TestLoader {
    fn load(&self) -> Result<Config> {
        Ok(Config::default())
    }

    fn load_from_path(&self, _path: &Path) -> Result<Config> {
        Ok(Config::default())
    }
}

#[test]
fn loader_returns_default_when_no_config() {
    let loader = TestLoader;
    let config = loader.load().unwrap();
    assert_eq!(config.default.max_lines, 500);
}
