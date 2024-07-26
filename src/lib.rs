use std::{env, fs, path::PathBuf};
use zed_extension_api::{self as zed, Result};

const SERVER_PATH: &str = "node_modules/dockerfile-language-server-nodejs/bin/docker-langserver";
const PACKAGE_NAME: &str = "dockerfile-language-server-nodejs";

struct DockerfileExtension {
    did_find_server: bool,
}

impl DockerfileExtension {
    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).map_or(false, |stat| stat.is_file())
    }

    fn server_script_path(&mut self, config: zed::LanguageServerConfig) -> Result<String> {
        let server_exists = self.server_exists();
        if self.did_find_server && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            &config.name,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                &config.name,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    if !self.server_exists() {
                        Err(format!(
                            "installed package '{PACKAGE_NAME}' did not contain expected path '{SERVER_PATH}'",
                        ))?;
                    }
                }
                Err(error) => {
                    if !self.server_exists() {
                        Err(error)?;
                    }
                }
            }
        }

        self.did_find_server = true;
        Ok(SERVER_PATH.to_string())
    }
}

impl zed::Extension for DockerfileExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    fn language_server_command(
        &mut self,
        config: zed::LanguageServerConfig,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.server_script_path(config)?;
        let (os, _) = zed::current_platform();
        let current_dir: PathBuf = match os {
            zed_extension_api::Os::Linux | zed_extension_api::Os::Mac => {
                env::current_dir().unwrap()
            }
            zed_extension_api::Os::Windows => {
                let dir_string = env::current_dir().unwrap().to_string_lossy().to_string();
                if let Some(result) = dir_string.strip_prefix('/') {
                    result.into()
                } else {
                    dir_string.into()
                }
            }
        };
        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![
                current_dir.join(&server_path).to_string_lossy().to_string(),
                "--stdio".to_string(),
            ],
            env: Default::default(),
        })
    }
}

zed::register_extension!(DockerfileExtension);
