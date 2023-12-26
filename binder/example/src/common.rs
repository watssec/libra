use std::path::Path;
use std::{env, fs};

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log::info;
use serde::de::DeserializeOwned;
use serde::Serialize;

use libra_shared::config::PATH_STUDIO;

lazy_static! {
    static ref FORCE: bool = matches!(env::var("FORCE"), Ok(val) if val == "1");
}

/// Common trait for workflow config
pub trait WorkflowConfig: Serialize + DeserializeOwned {
    /// Obtain the application name
    fn app() -> &'static str;

    /// Execute the profile
    fn run(self, workdir: &Path) -> Result<()>;
}

/// Run the workflows based on defined config files
pub fn execute<T: WorkflowConfig>() -> Result<()> {
    let app = T::app();

    // probe the configs
    let mut configs = vec![];
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_dir = base.join("src").join("apps").join(app).join("configs");
    for item in config_dir.read_dir()? {
        let item = item?;
        let path = item.path();
        if path.extension().map_or(false, |e| e == "json") {
            let content = fs::read_to_string(&path)?;
            let config: T = serde_json::from_str(&content)?;

            let filename = item
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("non-ascii string"))?;
            let name = filename
                .strip_suffix(".json")
                .expect("strip the .json suffix")
                .to_string();

            configs.push((name, config));
        }
    }

    // execute the configs one by one
    for (name, config) in configs {
        info!("Processing '{}' under config '{}'", app, name);

        // prepare the work directory
        let workdir = PATH_STUDIO.join("example").join(app).join(&name);
        if workdir.exists() && *FORCE {
            fs::remove_dir_all(&workdir)?;
        }
        if !workdir.exists() {
            fs::create_dir_all(&workdir)?;
        }

        // execute it
        config.run(&workdir)?;
    }
    Ok(())
}
