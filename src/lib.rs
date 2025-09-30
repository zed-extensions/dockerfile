use std::{env, fs};
use serde::{Deserialize, Serialize};
use zed_extension_api::{self as zed, Result};

const SERVER_PATH: &str = "node_modules/dockerfile-language-server-nodejs/bin/docker-langserver";
const PACKAGE_NAME: &str = "dockerfile-language-server-nodejs";

struct DockerfileExtension {
    did_find_server: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockerfileDebugConfig {
    // The absolute path to the context
    context_path: Option<String>,
    // The absolute path to the Dockerfile being built
    dockerfile: Option<String>,
    // This should only ever be "launch" as "attach" is unsupported
    request: Option<String>,
}

impl DockerfileExtension {
    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).map_or(false, |stat| stat.is_file())
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

impl zed::Extension for DockerfileExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed_extension_api::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.server_script_path(language_server_id)?;
        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![
                env::current_dir()
                    .unwrap()
                    .join(&server_path)
                    .to_string_lossy()
                    .to_string(),
                "--stdio".to_string(),
            ],
            env: Default::default(),
        })
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        value: zed::serde_json::Value,
    ) -> Result<zed::StartDebuggingRequestArgumentsRequest, String> {
        if adapter_name != "buildx-dockerfile" {
            return Err(format!(
                "Unexpected debug adapter launched in the Dockerfile extension \"{adapter_name}\""
            ));
        }

        value
            .get("request")
            .and_then(|request| {
                request.as_str().and_then(|s| match s {
                    "launch" => Some(zed::StartDebuggingRequestArgumentsRequest::Launch),
                    "attach" => None,
                    _ => None,
                })
            })
            .ok_or_else(|| {
                "Invalid request, expected `request` to be `launch`, `attach` or any other value is unsupported".into()
            })
    }

    fn dap_config_to_scenario(
        &mut self,
        zed_scenario: zed::DebugConfig,
    ) -> Result<zed::DebugScenario, String> {
        match zed_scenario.request {
            zed::DebugRequest::Launch(launch) => {
                let config = DockerfileDebugConfig {
                    dockerfile: Some(launch.program),
                    context_path: launch.cwd,
                    request: Some("launch".to_string()),
                };

                let config = zed::serde_json::to_value(config)
                    .map_err(|e| e.to_string())?
                    .to_string();

                Ok(zed::DebugScenario {
                    adapter: zed_scenario.adapter,
                    label: zed_scenario.label,
                    config,
                    tcp_connection: None,
                    build: None,
                })
            }
            zed::DebugRequest::Attach(_) => {
                return Err("attaching to a running build is not supported".to_string());
            }
        }
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: zed::DebugTaskDefinition,
        _user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> zed_extension_api::Result<zed::DebugAdapterBinary, String> {
        if adapter_name != "buildx-dockerfile" {
            return Err(format!(
                "Unexpected debug adapter launched in the Dockerfile extension \"{adapter_name}\""
            ));
        }

        Ok(zed::DebugAdapterBinary {
            command: Some("docker".to_string()),
            arguments: vec!["buildx".to_string(), "dap".to_string(), "build".to_string()],
            cwd: Some(worktree.root_path()),
            envs: vec![(String::from("BUILDX_EXPERIMENTAL"), String::from("1"))],
            request_args: zed::StartDebuggingRequestArguments {
                request: zed::StartDebuggingRequestArgumentsRequest::Launch,
                configuration: config.config
            },
            connection: None,
        })
    }
}

zed::register_extension!(DockerfileExtension);
