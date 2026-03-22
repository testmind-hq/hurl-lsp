import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as https from "node:https";
import type { ExtensionContext } from "vscode";

const BINARY_VERSION = "0.1.0";
const REPO = "yuchou87/hurl-lsp";

export async function ensureBinary(context: ExtensionContext): Promise<string> {
  const storageDir = path.join(context.globalStorageUri.fsPath, "bin");
  const binaryPath = path.join(storageDir, binaryName());

  await fs.promises.mkdir(storageDir, { recursive: true });

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  const target = detectTarget();
  const archiveName = `hurl-lsp-${BINARY_VERSION}-${target}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/v${BINARY_VERSION}/${archiveName}`;
  const archivePath = path.join(storageDir, archiveName);

  await download(url, archivePath);
  await extractTarGz(archivePath, storageDir);
  await fs.promises.chmod(binaryPath, 0o755);

  return binaryPath;
}

function binaryName(): string {
  return "hurl-lsp";
}

function detectTarget(): string {
  if (os.platform() !== "darwin") {
    throw new Error("Automatic download currently supports macOS only.");
  }
  if (os.arch() === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (os.arch() === "x64") {
    return "x86_64-apple-darwin";
  }
  throw new Error(`Unsupported macOS architecture: ${os.arch()}`);
}

function download(url: string, destination: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destination);
    https
      .get(url, (response) => {
        if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          file.close();
          fs.unlink(destination, () => download(response.headers.location!, destination).then(resolve, reject));
          return;
        }
        if (response.statusCode !== 200) {
          reject(new Error(`Failed to download hurl-lsp: HTTP ${response.statusCode ?? "unknown"}`));
          return;
        }
        response.pipe(file);
        file.on("finish", () => file.close(() => resolve()));
      })
      .on("error", (error) => {
        file.close();
        fs.unlink(destination, () => reject(error));
      });
  });
}

async function extractTarGz(archivePath: string, outputDir: string): Promise<void> {
  const { spawn } = await import("node:child_process");

  await new Promise<void>((resolve, reject) => {
    const child = spawn("tar", ["-xzf", archivePath, "-C", outputDir]);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`Failed to extract ${path.basename(archivePath)} (exit code ${code ?? "unknown"}).`));
      }
    });
    child.on("error", reject);
  });
}
