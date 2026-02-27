mod language_servers;

use serde::{Deserialize, Serialize};
use std::path::Path;
use zed_extension_api::{self as zed, settings::LspSettings, Result};

use crate::language_servers::{DockerLanguageServer, DockerfileLs};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockerfileDebugConfig {
    // The absolute path to the context
    context_path: Option<String>,
    // The absolute path to the Dockerfile being built
    dockerfile: Option<String>,
    // This should only ever be "launch" as "attach" is unsupported
    request: String,
    // args for the build, such as --build-arg ...
    #[serde(default)]
    args: Vec<String>,
    // Should the debugger suspend immediately on the first line
    stop_on_entry: Option<bool>,
    // The build stage to build
    target: Option<String>,
}

struct DockerfileExtension {
    dockerfile_ls: Option<DockerfileLs>,
    docker_language_server: Option<DockerLanguageServer>,
}

impl zed::Extension for DockerfileExtension {
    fn new() -> Self {
        Self {
            dockerfile_ls: None,
            docker_language_server: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            DockerfileLs::LANGUAGE_SERVER_ID => {
                let dockerfile_ls = self.dockerfile_ls.get_or_insert_with(DockerfileLs::new);
                dockerfile_ls.language_server_command(language_server_id, worktree)
            }
            DockerLanguageServer::LANGUAGE_SERVER_ID => {
                let docker_ls = self
                    .docker_language_server
                    .get_or_insert_with(DockerLanguageServer::new);
                docker_ls.language_server_command(language_server_id, worktree)
            }
            language_server_id => Err(format!("unknown language server: {language_server_id}")),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .map(|settings| settings.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .map(|settings| settings.settings)
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
                    request: "launch".to_string(),
                    args: launch.args,
                    stop_on_entry: zed_scenario.stop_on_entry,
                    target: None,
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
                Err("attaching to a running build is not supported".to_string())
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

        let configuration: serde_json::Value = serde_json::from_str(&config.config)
            .map_err(|e| format!("`config` is not a valid JSON: {e}"))?;
        let mut dockerfile_config: DockerfileDebugConfig =
            serde_json::from_value(configuration.clone())
                .map_err(|e| format!("`config` is not a valid Dockerfile config: {e}"))?;

        // Inject defaults if not set
        let root_path_str = worktree.root_path();
        if dockerfile_config.context_path.is_none() {
            dockerfile_config.context_path = Some(root_path_str.clone());
        }
        if dockerfile_config.dockerfile.is_none() {
            let root_path = Path::new(&root_path_str);
            let dockerfile_path = root_path.join("Dockerfile");
            let dockerfile_str = dockerfile_path
                .to_str()
                .ok_or_else(|| {
                    format!(
                        "Dockerfile path contains invalid UTF-8: {:?}",
                        dockerfile_path
                    )
                })?
                .to_owned();
            dockerfile_config.dockerfile = Some(dockerfile_str);
        }

        let final_config = serde_json::to_string(&dockerfile_config)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        let mut arguments = vec!["buildx".to_string(), "dap".to_string(), "build".to_string()];
        arguments.extend(dockerfile_config.args);

        Ok(zed::DebugAdapterBinary {
            command: Some("docker".to_string()),
            arguments,
            cwd: Some(worktree.root_path()),
            envs: vec![(String::from("BUILDX_EXPERIMENTAL"), String::from("1"))],
            request_args: zed::StartDebuggingRequestArguments {
                request: zed::StartDebuggingRequestArgumentsRequest::Launch,
                configuration: final_config,
            },
            connection: None,
        })
    }
}

zed::register_extension!(DockerfileExtension);
