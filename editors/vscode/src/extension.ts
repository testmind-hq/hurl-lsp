import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { Trace } from "vscode-jsonrpc";
import { ensureBinary } from "./download";

let client: LanguageClient | undefined;
let runtimeLogChannel: vscode.OutputChannel | undefined;
let requestLogChannel: vscode.OutputChannel | undefined;
let logNotificationDisposable: vscode.Disposable | undefined;
const RUN_COMMANDS = new Set(["hurl.runEntry", "hurl.runEntryWithVars", "hurl.runChain", "hurl.runFile"]);
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
  registerCommandForwarders(context);

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
  const serverOptions: ServerOptions = { command };
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
  appendRuntimeLog(`Language client started (trace=${traceSetting}).`);
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

function registerCommandForwarders(context: vscode.ExtensionContext): void {
  const commands = ["hurl.runEntry", "hurl.runEntryWithVars", "hurl.runChain", "hurl.runFile", "hurl.copyAsCurl"];
  for (const command of commands) {
    context.subscriptions.push(
      vscode.commands.registerCommand(command, async (...args: unknown[]) => {
        if (!client) {
          vscode.window.showWarningMessage("Hurl language client is not ready yet.");
          return;
        }
        const forwardedArgs = [...args];
        if (RUN_COMMANDS.has(command)) {
          const verbosity = vscode.workspace.getConfiguration("hurl").get<string>("run.verbosity", "verbose");
          forwardedArgs[2] = verbosity;
          appendRequestLog(`command=${command} verbosity=${verbosity}`);
        }
        try {
          await client.sendRequest("workspace/executeCommand", { command, arguments: forwardedArgs });
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          appendRuntimeLog(`Failed to execute command ${command}: ${message}`);
          vscode.window.showErrorMessage(`Hurl command failed: ${message}`);
        }
      }),
    );
  }
}
