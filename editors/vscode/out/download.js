"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.ensureBinary = ensureBinary;
const fs = __importStar(require("node:fs"));
const os = __importStar(require("node:os"));
const path = __importStar(require("node:path"));
const https = __importStar(require("node:https"));
const BINARY_VERSION = "0.1.0";
const REPO = "yuchou87/hurl-lsp";
async function ensureBinary(context) {
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
function binaryName() {
    return "hurl-lsp";
}
function detectTarget() {
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
function download(url, destination) {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(destination);
        https
            .get(url, (response) => {
            if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
                file.close();
                fs.unlink(destination, () => download(response.headers.location, destination).then(resolve, reject));
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
async function extractTarGz(archivePath, outputDir) {
    const { spawn } = await Promise.resolve().then(() => __importStar(require("node:child_process")));
    await new Promise((resolve, reject) => {
        const child = spawn("tar", ["-xzf", archivePath, "-C", outputDir]);
        child.on("exit", (code) => {
            if (code === 0) {
                resolve();
            }
            else {
                reject(new Error(`Failed to extract ${path.basename(archivePath)} (exit code ${code ?? "unknown"}).`));
            }
        });
        child.on("error", reject);
    });
}
//# sourceMappingURL=download.js.map