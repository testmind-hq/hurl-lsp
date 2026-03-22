import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { Trace } from "vscode-jsonrpc";
import { ensureBinary } from "./download";

let client: LanguageClient | undefined;
let runtimeLogChannel: vscode.OutputChannel | undefined;
let requestLogChannel: vscode.OutputChannel | undefined;
let logNotificationDisposable: vscode.Disposable | undefined;
const REQUEST_LOG_PREFIX = "[hurl-request] ";

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  runtimeLogChannel = vscode.window.createOutputChannel("Hurl Runtime Log");
  requestLogChannel = vscode.window.createOutputChannel("Hurl Request Log");
  context.subscriptions.push(runtimeLogChannel);
  context.subscriptions.push(requestLogChannel);
  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.restartLanguageServer", async () => {
      await restart(context);
    }),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.showLog", () => {
      runtimeLogChannel?.show(true);
    }),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.showRequestLog", () => {
      requestLogChannel?.show(true);
    }),
  );

  await start(context);
}

export async function deactivate(): Promise<void> {
  if (logNotificationDisposable) {
    logNotificationDisposable.dispose();
    logNotificationDisposable = undefined;
  }
  if (client) {
    await client.stop();
    client = undefined;
  }
}

async function restart(context: vscode.ExtensionContext): Promise<void> {
  await deactivate();
  await start(context);
}

async function start(context: vscode.ExtensionContext): Promise<void> {
  const configuredPath = vscode.workspace.getConfiguration("hurl").get<string>("server.path");
  let command = configuredPath?.trim();

  if (!command) {
    try {
      const binaryVersion = String(context.extension.packageJSON.version ?? "").trim();
      if (!binaryVersion) {
        throw new Error("Missing extension version for release binary resolution.");
      }
      command = await ensureBinary(context, binaryVersion);
      appendRuntimeLog(`Using server binary: ${command}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendRuntimeLog(`Failed to resolve server binary: ${message}`);
      void vscode.window.showErrorMessage(
        `Unable to start hurl-lsp automatically. Set hurl.server.path to a local binary. ${message}`,
      );
      return;
    }
  } else {
    appendRuntimeLog(`Using configured server path: ${command}`);
  }

  const traceSetting = vscode.workspace.getConfiguration("hurl").get<string>("server.trace", "off");
  const runVerbosity = vscode.workspace.getConfiguration("hurl").get<string>("run.verbosity", "verbose");
  const serverOptions: ServerOptions = {
    command,
    options: {
      env: {
        ...process.env,
        HURL_RUN_VERBOSITY: runVerbosity,
      },
    },
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "hurl" }],
    outputChannelName: "Hurl Language Server",
  };

  client = new LanguageClient("hurl-lsp", "Hurl Language Server", serverOptions, clientOptions);
  context.subscriptions.push(client);
  if (logNotificationDisposable) {
    logNotificationDisposable.dispose();
    logNotificationDisposable = undefined;
  }
  logNotificationDisposable = client.onNotification("window/logMessage", (params: { message?: string }) => {
    if (params?.message) {
      if (params.message.startsWith(REQUEST_LOG_PREFIX)) {
        appendRequestLog(params.message.slice(REQUEST_LOG_PREFIX.length));
      } else {
        appendRuntimeLog(params.message);
      }
    }
  });
  context.subscriptions.push(logNotificationDisposable);

  await client.start();
  client.setTrace(toTrace(traceSetting));
  appendRuntimeLog(`Language client started (trace=${traceSetting}, runVerbosity=${runVerbosity}).`);
}

function toTrace(value: string): Trace {
  switch (value) {
    case "messages":
      return Trace.Messages;
    case "verbose":
      return Trace.Verbose;
    default:
      return Trace.Off;
  }
}

function appendRuntimeLog(message: string): void {
  if (!runtimeLogChannel) {
    return;
  }
  const ts = new Date().toISOString();
  runtimeLogChannel.appendLine(`[${ts}] ${message}`);
}

function appendRequestLog(message: string): void {
  if (!requestLogChannel) {
    return;
  }
  const ts = new Date().toISOString();
  requestLogChannel.appendLine(`[${ts}] ${message}`);
}
