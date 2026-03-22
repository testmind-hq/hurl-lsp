import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { Trace } from "vscode-jsonrpc";
import { ensureBinary } from "./download";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.restartLanguageServer", async () => {
      await restart(context);
    }),
  );

  await start(context);
}

export async function deactivate(): Promise<void> {
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
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      void vscode.window.showErrorMessage(
        `Unable to start hurl-lsp automatically. Set hurl.server.path to a local binary. ${message}`,
      );
      return;
    }
  }

  const traceSetting = vscode.workspace.getConfiguration("hurl").get<string>("server.trace", "off");
  const serverOptions: ServerOptions = { command };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "hurl" }],
    outputChannelName: "Hurl Language Server",
  };

  client = new LanguageClient("hurl-lsp", "Hurl Language Server", serverOptions, clientOptions);
  context.subscriptions.push(client);
  await client.start();
  client.setTrace(toTrace(traceSetting));
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
