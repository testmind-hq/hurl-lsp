import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { Trace } from "vscode-jsonrpc";
import { ensureBinary } from "./download";
import { exportActiveHurlAsMarkdown } from "./markdownExport";

let client: LanguageClient | undefined;
let runtimeLogChannel: vscode.OutputChannel | undefined;
let requestLogChannel: vscode.OutputChannel | undefined;
let logNotificationDisposable: vscode.Disposable | undefined;
const REQUEST_LOG_PREFIX = "[hurl-request] ";
let requestRuns: string[] = [];
let activeRunIndex = -1;

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
  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.exportAsMarkdown", async () => {
      await exportActiveHurlAsMarkdown();
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
  const runLogMaxChars = vscode.workspace.getConfiguration("hurl").get<number>("run.log.maxCharsPerRun", 6000);
  const serverOptions: ServerOptions = {
    command,
    options: {
      env: {
        ...process.env,
        HURL_RUN_VERBOSITY: runVerbosity,
        HURL_RUN_LOG_MAX_CHARS: String(runLogMaxChars),
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
  appendRuntimeLog(
    `Language client started (trace=${traceSetting}, runVerbosity=${runVerbosity}, runLogMaxChars=${runLogMaxChars}).`,
  );
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
  const channel = requestLogChannel;
  const config = vscode.workspace.getConfiguration("hurl");
  const clearOnRun = config.get<boolean>("run.log.clearOnRun", false);
  const maxRuns = Math.max(1, config.get<number>("run.log.maxRuns", 3));
  const maxCharsPerRun = Math.max(0, config.get<number>("run.log.maxCharsPerRun", 6000));
  const ts = new Date().toISOString();
  const fullLine = `[${ts}] ${message}`;
  const line = truncateLogChunk(fullLine, maxCharsPerRun);

  if (message.startsWith("run target=")) {
    if (clearOnRun) {
      requestRuns = [];
      activeRunIndex = -1;
    }
    requestRuns.push(line);
    while (requestRuns.length > maxRuns) {
      requestRuns.shift();
    }
    activeRunIndex = requestRuns.length - 1;
  } else {
    if (activeRunIndex < 0 || activeRunIndex >= requestRuns.length) {
      requestRuns.push(line);
      while (requestRuns.length > maxRuns) {
        requestRuns.shift();
      }
      activeRunIndex = requestRuns.length - 1;
    } else {
      requestRuns[activeRunIndex] = `${requestRuns[activeRunIndex]}\n${line}`;
    }
  }

  channel.clear();
  requestRuns.forEach((chunk, index) => {
    if (index > 0) {
      channel.appendLine("");
      channel.appendLine("────────────────────────────────────────────────────────");
    }
    channel.appendLine(chunk);
  });
}

function truncateLogChunk(text: string, maxChars: number): string {
  if (maxChars <= 0) {
    return text;
  }
  if (text.length <= maxChars) {
    return text;
  }
  return `${text.slice(0, maxChars)}… [truncated]`;
}
