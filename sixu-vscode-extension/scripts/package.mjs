/**
 * Package helper script for building platform-specific .vsix files.
 *
 * Usage:
 *   node scripts/package.mjs --target <vscode-platform> --binary <path-to-lsp-binary>
 *
 * Examples:
 *   node scripts/package.mjs --target win32-x64 --binary ../target/release/sixu-lsp.exe
 *   node scripts/package.mjs --target darwin-arm64 --binary ../target/aarch64-apple-darwin/release/sixu-lsp
 *   node scripts/package.mjs --target linux-x64 --binary ../target/x86_64-unknown-linux-gnu/release/sixu-lsp
 */

import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const extensionRoot = path.resolve(__dirname, '..');

function parseArgs() {
  const args = process.argv.slice(2);
  let target = null;
  let binary = null;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--target' && args[i + 1]) {
      target = args[++i];
    } else if (args[i] === '--binary' && args[i + 1]) {
      binary = args[++i];
    }
  }

  if (!target || !binary) {
    console.error('Usage: node scripts/package.mjs --target <vscode-platform> --binary <path-to-lsp-binary>');
    console.error('');
    console.error('Supported targets: win32-x64, darwin-x64, darwin-arm64, linux-x64');
    process.exit(1);
  }

  return { target, binary };
}

const VALID_TARGETS = ['win32-x64', 'darwin-x64', 'darwin-arm64', 'linux-x64'];

function main() {
  const { target, binary } = parseArgs();

  if (!VALID_TARGETS.includes(target)) {
    console.error(`Invalid target: ${target}`);
    console.error(`Valid targets: ${VALID_TARGETS.join(', ')}`);
    process.exit(1);
  }

  // Verify the binary exists
  const binaryPath = path.resolve(binary);
  if (!fs.existsSync(binaryPath)) {
    console.error(`Binary not found: ${binaryPath}`);
    process.exit(1);
  }

  // Prepare server directory
  const serverDir = path.join(extensionRoot, 'server');
  if (!fs.existsSync(serverDir)) {
    fs.mkdirSync(serverDir, { recursive: true });
  }

  // Determine the output binary name
  const isWindows = target.startsWith('win32');
  const outputBinaryName = isWindows ? 'sixu-lsp.exe' : 'sixu-lsp';
  const destPath = path.join(serverDir, outputBinaryName);

  // Copy binary
  console.log(`Copying ${binaryPath} -> ${destPath}`);
  fs.copyFileSync(binaryPath, destPath);

  // Make executable on non-Windows
  if (!isWindows) {
    fs.chmodSync(destPath, 0o755);
  }

  // Build the extension JS
  console.log('Building extension with esbuild...');
  execSync('node esbuild.mjs --production', { cwd: extensionRoot, stdio: 'inherit' });

  // Package with vsce
  console.log(`Packaging for ${target}...`);
  execSync(`npx @vscode/vsce package --no-dependencies --target ${target}`, { cwd: extensionRoot, stdio: 'inherit' });

  // Clean up server directory
  fs.rmSync(serverDir, { recursive: true, force: true });

  console.log(`\nDone! .vsix file created for ${target}`);
}

main();
