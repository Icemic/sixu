import * as path from 'path';
import { workspace, window } from 'vscode';

import { LanguageClient, TransportKind } from 'vscode-languageclient/node';

let client;

/**
 * Resolve the LSP server binary path.
 *
 * Resolution order:
 * 1. Environment variable SIXU_LSP_PATH (for local development / debugging)
 * 2. Bundled binary inside the extension's server/ directory
 */
function resolveServerPath(context) {
  // Allow overriding via environment variable for development
  const envPath = process.env.SIXU_LSP_PATH;
  if (envPath) {
    return envPath;
  }

  const binaryName = process.platform === 'win32' ? 'sixu-lsp.exe' : 'sixu-lsp';
  return context.asAbsolutePath(path.join('server', binaryName));
}

export function activate(context) {
  const serverPath = resolveServerPath(context);

  const fs = require('fs');
  if (!fs.existsSync(serverPath)) {
    window.showErrorMessage(
      `Sixu LSP server binary not found at: ${serverPath}. ` +
      `Please reinstall the extension or set the SIXU_LSP_PATH environment variable.`
    );
    return;
  }

  console.log(`Sixu LSP Server Path: ${serverPath}`);

  const serverOptions = {
    run: { command: serverPath, transport: TransportKind.stdio },
    debug: { command: serverPath, transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'sixu' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/.clientrc'),
    },
  };

  client = new LanguageClient('sixuLanguageServer', 'Sixu Language Server', serverOptions, clientOptions);

  client.start();
}

export function deactivate() {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
