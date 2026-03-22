import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as https from "node:https";
import { spawn } from "node:child_process";
import type { ExtensionContext } from "vscode";

const REPO = "testmind-hq/hurl-lsp";

export async function ensureBinary(context: ExtensionContext, binaryVersion: string): Promise<string> {
  const storageDir = path.join(context.globalStorageUri.fsPath, "bin");
  const binaryPath = path.join(storageDir, binaryName());

  await fs.promises.mkdir(storageDir, { recursive: true });

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  const target = detectTarget();
  const asset = releaseAssetForTarget(target);
  const archiveName = `hurl-lsp-${binaryVersion}-${target}.${asset.extension}`;
  const url = `https://github.com/${REPO}/releases/download/v${binaryVersion}/${archiveName}`;
  const archivePath = path.join(storageDir, archiveName);

  await download(url, archivePath);
  await extractArchive(archivePath, storageDir, asset.extension);
  if (os.platform() !== "win32") {
    await fs.promises.chmod(binaryPath, 0o755);
  }

  return binaryPath;
}

function binaryName(): string {
  return os.platform() === "win32" ? "hurl-lsp.exe" : "hurl-lsp";
}

function detectTarget(): string {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (platform === "darwin" && arch === "x64") {
    return "x86_64-apple-darwin";
  }
  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }
  if (platform === "win32" && arch === "x64") {
    return "x86_64-pc-windows-msvc";
  }
  throw new Error(`Unsupported platform/arch combination: ${platform}/${arch}`);
}

function releaseAssetForTarget(target: string): { extension: "tar.gz" | "zip" } {
  if (target.endsWith("windows-msvc")) {
    return { extension: "zip" };
  }
  return { extension: "tar.gz" };
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

async function extractArchive(
  archivePath: string,
  outputDir: string,
  extension: "tar.gz" | "zip",
): Promise<void> {
  if (extension === "zip") {
    await extractZip(archivePath, outputDir);
    return;
  }
  await extractTarGz(archivePath, outputDir);
}

async function extractTarGz(archivePath: string, outputDir: string): Promise<void> {
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

async function extractZip(archivePath: string, outputDir: string): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const command = `Expand-Archive -Path '${archivePath.replace(/'/g, "''")}' -DestinationPath '${outputDir.replace(/'/g, "''")}' -Force`;
    const child = spawn("powershell", ["-NoProfile", "-Command", command]);
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
