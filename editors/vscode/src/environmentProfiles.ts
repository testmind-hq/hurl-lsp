import * as vscode from "vscode";
import { ActiveEnvironmentProfile, AUTO_PROFILE, normalizeProfiles, resolveActiveProfile } from "./environmentProfileModel";

const STATE_PREFIX = "hurl.environment.profile.";

export class EnvironmentProfileController implements vscode.Disposable {
  private readonly statusBar: vscode.StatusBarItem;
  private readonly emitter = new vscode.EventEmitter<{ folderUri: string; profile: ActiveEnvironmentProfile }>();
  private readonly disposables: vscode.Disposable[] = [];
  readonly onDidChange = this.emitter.event;

  constructor(private readonly context: vscode.ExtensionContext) {
    this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);
    this.statusBar.command = "hurl.selectEnvironmentProfile";
    this.statusBar.tooltip = "Select the Hurl environment profile for this workspace folder";
    this.disposables.push(
      this.statusBar,
      this.emitter,
      vscode.commands.registerCommand("hurl.selectEnvironmentProfile", () => this.selectForActiveEditor()),
      vscode.window.onDidChangeActiveTextEditor(() => this.refresh()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("hurl.environment")) this.refresh();
      }),
    );
    this.refresh();
  }

  activeFor(uri: vscode.Uri): ActiveEnvironmentProfile {
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    if (!folder) return { name: AUTO_PROFILE, files: [] };
    const config = vscode.workspace.getConfiguration("hurl", folder.uri);
    const profiles = normalizeProfiles(config.get("environment.profiles", {}));
    const defaultProfile = config.get<string>("environment.defaultProfile", AUTO_PROFILE);
    const selected = this.context.workspaceState.get<string>(`${STATE_PREFIX}${folder.uri.toString()}`);
    return resolveActiveProfile(folder.uri.toString(), selected ? { [folder.uri.toString()]: selected } : {}, defaultProfile, profiles);
  }

  async selectForActiveEditor(): Promise<void> {
    const uri = vscode.window.activeTextEditor?.document.uri;
    const folder = uri && vscode.workspace.getWorkspaceFolder(uri);
    if (!uri || !folder) {
      void vscode.window.showWarningMessage("Open a Hurl file inside a workspace folder to select an environment.");
      return;
    }
    const config = vscode.workspace.getConfiguration("hurl", folder.uri);
    const profiles = normalizeProfiles(config.get("environment.profiles", {}));
    const active = this.activeFor(uri);
    const choices = [
      { label: AUTO_PROFILE, description: "Automatically discover variable files" },
      ...Object.entries(profiles).map(([name, files]) => ({ label: name, description: files.join(" → ") || "No variable files" })),
    ];
    const picked = await vscode.window.showQuickPick(choices, {
      title: `Hurl environment — ${folder.name}`,
      placeHolder: active.name,
    });
    if (!picked) return;
    await this.context.workspaceState.update(`${STATE_PREFIX}${folder.uri.toString()}`, picked.label);
    const profile = this.activeFor(uri);
    this.refresh();
    this.emitter.fire({ folderUri: folder.uri.toString(), profile });
  }

  refresh(): void {
    const editor = vscode.window.activeTextEditor;
    if (!editor || (editor.document.languageId !== "hurl" && !editor.document.fileName.endsWith(".hurl"))) {
      this.statusBar.hide();
      return;
    }
    const profile = this.activeFor(editor.document.uri);
    this.statusBar.text = `$(server-environment) Hurl: ${profile.name}`;
    this.statusBar.show();
  }

  dispose(): void {
    for (const disposable of this.disposables) disposable.dispose();
  }
}
