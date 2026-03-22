import * as vscode from "vscode";

type OutlineNode = {
  label: string;
  uri: vscode.Uri;
  line?: number;
  children?: OutlineNode[];
};

export class HurlOutlineProvider implements vscode.TreeDataProvider<OutlineNode> {
  private readonly onDidChangeTreeDataEmitter = new vscode.EventEmitter<OutlineNode | undefined>();
  readonly onDidChangeTreeData = this.onDidChangeTreeDataEmitter.event;

  refresh(): void {
    this.onDidChangeTreeDataEmitter.fire(undefined);
  }

  getTreeItem(element: OutlineNode): vscode.TreeItem {
    const hasChildren = Boolean(element.children && element.children.length > 0);
    const item = new vscode.TreeItem(
      element.label,
      hasChildren ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.None,
    );
    if (typeof element.line === "number") {
      item.contextValue = "hurlOutlineEntry";
      item.command = {
        title: "Open",
        command: "vscode.open",
        arguments: [element.uri, { selection: new vscode.Range(element.line, 0, element.line, 0) }],
      };
    } else {
      item.contextValue = "hurlOutlineGroup";
    }
    return item;
  }

  async getChildren(element?: OutlineNode): Promise<OutlineNode[]> {
    if (element?.children) {
      return element.children;
    }
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      return [];
    }
    const doc = editor.document;
    if (doc.languageId !== "hurl" && !doc.fileName.endsWith(".hurl")) {
      return [];
    }
    const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
      "vscode.executeDocumentSymbolProvider",
      doc.uri,
    );
    if (!symbols || symbols.length === 0) {
      return [];
    }
    return symbols.map((symbol) => toNode(doc.uri, symbol));
  }
}

function toNode(uri: vscode.Uri, symbol: vscode.DocumentSymbol): OutlineNode {
  const children = (symbol.children ?? []).map((child) => toNode(uri, child));
  const hasChildren = children.length > 0;
  return {
    label: symbol.name,
    uri,
    line: hasChildren ? undefined : symbol.range.start.line,
    children: hasChildren ? children : undefined,
  };
}
