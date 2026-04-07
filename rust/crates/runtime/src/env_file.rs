use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum EnvFileError {
    Parse { path: String, message: String },
}

impl std::fmt::Display for EnvFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { path, message } => {
                write!(f, "failed to load .env from {path}: {message}")
            }
        }
    }
}

impl std::error::Error for EnvFileError {}

pub fn load_dotenv_for(workspace_root: &Path) -> Result<bool, EnvFileError> {
    let env_path = workspace_root.join(".env");
    if !env_path.is_file() {
        return Ok(false);
    }

    let mut loaded_any = false;
    let iter = dotenvy::from_path_iter(&env_path).map_err(|error| EnvFileError::Parse {
        path: env_path.display().to_string(),
        message: error.to_string(),
    })?;

    for item in iter {
        let (key, value) = item.map_err(|error| EnvFileError::Parse {
            path: env_path.display().to_string(),
            message: error.to_string(),
        })?;
        if env::var_os(&key).is_some() {
            continue;
        }
        env::set_var(key, value);
        loaded_any = true;
    }

    Ok(loaded_any)
}

pub fn load_dotenv_upwards(start_dir: &Path) -> Result<Option<PathBuf>, EnvFileError> {
    let mut current = Some(start_dir);
    while let Some(path) = current {
        if path.join(".env").is_file() {
            let _ = load_dotenv_for(path)?;
            return Ok(Some(path.to_path_buf()));
        }
        current = path.parent();
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{load_dotenv_for, load_dotenv_upwards};
    use crate::test_env_lock;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let original = std::env::var(key).ok();
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_millis();
        std::env::temp_dir().join(format!("claw-runtime-{name}-{millis}"))
    }

    #[test]
    fn loads_env_file_without_overriding_existing_process_env() {
        let _lock = test_env_lock();
        let root = temp_root("dotenv");
        fs::create_dir_all(&root).expect("temp root should exist");
        fs::write(
            root.join(".env"),
            "OPENAI_BASE_URL=https://example-openai.local/v1\nOPENAI_API_KEY=from-dotenv\n",
        )
        .expect(".env should write");
        let _existing = EnvVarGuard::set("OPENAI_API_KEY", Some("from-process"));
        let _base_url = EnvVarGuard::set("OPENAI_BASE_URL", None);

        let loaded = load_dotenv_for(&root).expect(".env should load");

        assert!(loaded);
        assert_eq!(
            std::env::var("OPENAI_BASE_URL").as_deref(),
            Ok("https://example-openai.local/v1")
        );
        assert_eq!(
            std::env::var("OPENAI_API_KEY").as_deref(),
            Ok("from-process")
        );

        fs::remove_dir_all(root).expect("temp root should clean up");
    }

    #[test]
    fn returns_false_when_env_file_is_missing() {
        let _lock = test_env_lock();
        let root = temp_root("dotenv-missing");
        fs::create_dir_all(&root).expect("temp root should exist");

        let loaded = load_dotenv_for(&root).expect("missing .env should not fail");

        assert!(!loaded);
        fs::remove_dir_all(root).expect("temp root should clean up");
    }

    #[test]
    fn finds_env_file_in_parent_directory() {
        let _lock = test_env_lock();
        let root = temp_root("dotenv-parent");
        let child = root.join("rust");
        fs::create_dir_all(&child).expect("child root should exist");
        fs::write(root.join(".env"), "PERPLEXITY_API_KEY=from-parent\n")
            .expect(".env should write");
        let _perplexity = EnvVarGuard::set("PERPLEXITY_API_KEY", None);

        let loaded_from = load_dotenv_upwards(&child).expect(".env should load from parent");

        assert_eq!(loaded_from.as_deref(), Some(root.as_path()));
        assert_eq!(
            std::env::var("PERPLEXITY_API_KEY").as_deref(),
            Ok("from-parent")
        );

        fs::remove_dir_all(root).expect("temp root should clean up");
    }
}
