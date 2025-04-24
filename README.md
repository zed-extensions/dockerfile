# Dockerfile Zed Extension

- Tree Sitter: [tree-sitter-dockerfile](https://github.com/camdencheek/tree-sitter-dockerfile)
- Language Server: [dockerfile-language-server](https://github.com/rcjsuen/dockerfile-language-server)

## Configuration

To support matching filenames other than `Dockerfile` you can add [`file_types`](https://zed.dev/docs/configuring-zed#file-types) to your Zed project or user settings:

```json
{
  "file_types": {
    "Dockerfile": [ "Dockerfile.*" ]
  }
}
```
