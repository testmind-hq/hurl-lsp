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
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const vscode_jsonrpc_1 = require("vscode-jsonrpc");
const download_1 = require("./download");
let client;
async function activate(context) {
    context.subscriptions.push(vscode.commands.registerCommand("hurl.restartLanguageServer", async () => {
        await restart(context);
    }));
    await start(context);
}
async function deactivate() {
    if (client) {
        await client.stop();
        client = undefined;
    }
}
async function restart(context) {
    await deactivate();
    await start(context);
}
async function start(context) {
    const configuredPath = vscode.workspace.getConfiguration("hurl").get("server.path");
    let command = configuredPath?.trim();
    if (!command) {
        try {
            command = await (0, download_1.ensureBinary)(context);
        }
        catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            void vscode.window.showErrorMessage(`Unable to start hurl-lsp automatically. Configure hurl.server.path manually. ${message}`);
            return;
        }
    }
    const traceSetting = vscode.workspace.getConfiguration("hurl").get("server.trace", "off");
    const serverOptions = { command };
    const clientOptions = {
        documentSelector: [{ scheme: "file", language: "hurl" }],
        outputChannelName: "Hurl Language Server",
    };
    client = new node_1.LanguageClient("hurl-lsp", "Hurl Language Server", serverOptions, clientOptions);
    context.subscriptions.push(client);
    await client.start();
    client.setTrace(toTrace(traceSetting));
}
function toTrace(value) {
    switch (value) {
        case "messages":
            return vscode_jsonrpc_1.Trace.Messages;
        case "verbose":
            return vscode_jsonrpc_1.Trace.Verbose;
        default:
            return vscode_jsonrpc_1.Trace.Off;
    }
}
//# sourceMappingURL=extension.js.map