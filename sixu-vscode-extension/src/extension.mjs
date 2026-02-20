import * as path from 'path';
import { workspace } from 'vscode';

import { LanguageClient, TransportKind } from 'vscode-languageclient/node';

let client;

export function activate(context) {
  // The server is implemented in node
  // let serverModule = context.asAbsolutePath(path.join('server', 'out', 'server.js'));
  // The debug options for the server
  // --inspect=6009: runs the server in Node's Inspector mode so VS Code can attach to the server for debugging
  // let debugOptions = { execArgv: ['--nolazy', '--inspect=6009'] };

  // For now, we assume the server binary is in the target/debug directory relative to the workspace root
  // In a real extension, we would bundle the binary or download it
  const serverPath = context.asAbsolutePath(path.join('..', 'target', 'debug', 'sixu-lsp.exe'));

  console.log(`Sixu LSP Server Path: ${serverPath}`);

  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  const serverOptions = {
    run: { command: serverPath, transport: TransportKind.stdio },
    debug: { command: serverPath, transport: TransportKind.stdio },
  };

  // Options to control the language client
  const clientOptions = {
    // Register the server for plain text documents
    documentSelector: [{ scheme: 'file', language: 'sixu' }],
    synchronize: {
      // Notify the server about file changes to '.clientrc files contained in the workspace
      fileEvents: workspace.createFileSystemWatcher('**/.clientrc'),
    },
  };

  // Create the language client and start the client.
  client = new LanguageClient('sixuLanguageServer', 'Sixu Language Server', serverOptions, clientOptions);

  // Start the client. This will also launch the server
  client.start();
}

export function deactivate() {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
