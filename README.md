# Dockerfile Zed Extension

- Tree Sitter: [tree-sitter-dockerfile](https://github.com/camdencheek/tree-sitter-dockerfile)
- Language Server: [dockerfile-language-server](https://github.com/rcjsuen/dockerfile-language-server)

## Configuration

To support matching filenames other than `Dockerfile` you can add [`file_types`](https://zed.dev/docs/configuring-zed#file-types) to your Zed project or user settings:

```json
{
  "file_types": {
    "Dockerfile": ["Dockerfile.*"]
  }
}
```

## Debugging

The extension supports debugging Dockerfile builds with [Buildx](https://github.com/docker/buildx). The minimal required version of Buildx is v0.28.0. To get Buildx, we recommend installing or updating [Docker Desktop](https://docs.docker.com/install/). You may alternatively install Buildx manually by following the instructions [here](https://github.com/docker/buildx?tab=readme-ov-file#manual-download).

You can validate your Buildx installation by running `BUILDX_EXPERIMENTAL=1 docker buildx dap`.

You can create a debug configuration by modifying your project's `.zed/debug.json`.

```json
{
  "label": "Docker: Build", // required, configurable
  "adapter": "buildx-dockerfile", // required, must not be modified
  "request": "launch", // required, must not be modified
  "contextPath": "/home/username/worktree", // optional, defaults to ${ZED_WORKTREE_ROOT}
  "dockerfile": "/home/username/worktree/Dockerfile", // optional, defaults to ${ZED_WORKTREE_ROOT}/Dockerfile
  "target": "test", // optional, should be a build stage in the Dockerfile
  "stopOnEntry": true, // if the debugger should suspend on the first line, defaults to false
  "args": [
    // additional arguments for the build command
    "--build-arg",
    "NODE_ENV=development"
  ]
}
```

While a build has been suspended, you can evaluate `exec` to open a shell into the Docker image that has been built up to that point in time.
