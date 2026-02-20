# SiXu Language Support

VS Code extension for the [SiXu (思绪)](https://github.com/Icemic/sixu) visual novel scripting language.

## Features

- Syntax highlighting for `.sixu` files
- LSP support: completions, diagnostics, go-to-definition, formatting

## Installation

### From GitHub Releases (recommended)

1. Go to the [Releases page](https://github.com/Icemic/sixu/releases)
2. Download the `.vsix` file matching your platform (e.g. `sixu-vscode-extension-win32-x64-0.1.0.vsix`)
3. Install via command line:
   ```bash
   code --install-extension sixu-vscode-extension-<platform>-<version>.vsix
   ```
   Or in VS Code: Extensions view → `...` menu → "Install from VSIX..."

### Supported Platforms

| Platform | File |
|----------|------|
| Windows x64 | `*-win32-x64-*.vsix` |
| macOS x64 (Intel) | `*-darwin-x64-*.vsix` |
| macOS ARM64 (Apple Silicon) | `*-darwin-arm64-*.vsix` |
| Linux x64 | `*-linux-x64-*.vsix` |

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- [Yarn](https://yarnpkg.com/) (via Corepack)

### Local Development

1. Build the LSP server:
   ```bash
   cargo build -p sixu-lsp
   ```

2. Install extension dependencies:
   ```bash
   cd sixu-vscode-extension
   yarn install
   ```

3. Set the environment variable to point to the debug binary:
   ```bash
   # Windows (PowerShell)
   $env:SIXU_LSP_PATH = "$PWD\target\debug\sixu-lsp.exe"

   # macOS / Linux
   export SIXU_LSP_PATH="$PWD/target/debug/sixu-lsp"
   ```

4. Press F5 in VS Code to launch the Extension Development Host.

### Building a Local VSIX

```bash
cd sixu-vscode-extension

# Example: package for Windows x64
node scripts/package.mjs --target win32-x64 --binary ../target/release/sixu-lsp.exe
```
