use std::{env, fs};

use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, Result};

const SERVER_PATH: &str = "node_modules/dockerfile-language-server-nodejs/bin/docker-langserver";
const PACKAGE_NAME: &str = "dockerfile-language-server-nodejs";

pub struct DockerfileLs {
    did_find_server: bool,
}

impl DockerfileLs {
    pub const LANGUAGE_SERVER_ID: &'static str = "dockerfile-language-server";

    pub fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).is_ok_and(|stat| stat.is_file())
    }

    pub fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        let env = binary_settings
            .as_ref()
            .and_then(|s| s.env.as_ref())
            .map(|env| env.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        if let Some(path) = binary_settings.as_ref().and_then(|s| s.path.clone()) {
            let args = binary_settings
                .as_ref()
                .and_then(|s| s.arguments.clone())
                .unwrap_or_else(|| vec!["--stdio".to_string()]);

            return Ok(zed::Command {
                command: path,
                args,
                env,
            });
        }

        let server_path = self.server_script_path(language_server_id)?;

        let default_args = vec![
            env::current_dir()
                .unwrap()
                .join(&server_path)
                .to_string_lossy()
                .to_string(),
            "--stdio".to_string(),
        ];

        let args = binary_settings
            .as_ref()
            .and_then(|s| s.arguments.clone())
            .unwrap_or(default_args);

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args,
            env,
        })
    }

    fn server_script_path(&mut self, language_server_id: &zed::LanguageServerId) -> Result<String> {
        let server_exists = self.server_exists();
        if self.did_find_server && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
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
