import * as vscode from "vscode";
import {
  ActiveEnvironmentProfile,
  AUTO_PROFILE,
  discoverConventionProfiles,
  mergeEnvironmentProfiles,
  normalizeProfiles,
  profileWatchPaths,
  resolveActiveProfile,
} from "./environmentProfileModel";

const STATE_PREFIX = "hurl.environment.profile.";

export class EnvironmentProfileController implements vscode.Disposable {
  private readonly statusBar: vscode.StatusBarItem;
  private readonly emitter = new vscode.EventEmitter<{ folderUri: string; profile: ActiveEnvironmentProfile }>();
  private readonly disposables: vscode.Disposable[] = [];
  private profileWatchers: vscode.Disposable[] = [];
  private readonly discoveredProfiles = new Map<string, Record<string, string[]>>();
  readonly onDidChange = this.emitter.event;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly onVariableFileChange?: (uri: vscode.Uri, type: vscode.FileChangeType) => void,
  ) {
    this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);
    this.statusBar.command = "hurl.selectEnvironmentProfile";
    this.statusBar.tooltip = "Select the Hurl environment profile for this workspace folder";
    this.disposables.push(
      this.statusBar,
      this.emitter,
      vscode.commands.registerCommand("hurl.selectEnvironmentProfile", () => this.selectForActiveEditor()),
      vscode.commands.registerCommand("hurl.createEnvironmentFile", () => this.createForActiveEditor()),
      vscode.commands.registerCommand("hurl.openEnvironmentFiles", () => this.openForActiveEditor()),
      vscode.window.onDidChangeActiveTextEditor(() => this.refresh()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("hurl.environment")) {
          this.rebuildProfileWatchers();
          this.refresh();
          this.notifyAll();
        }
      }),
      vscode.workspace.onDidChangeWorkspaceFolders(() => {
        void this.refreshDiscoveredProfiles().then(() => {
          this.rebuildProfileWatchers();
          this.refresh();
          this.notifyAll();
        });
      }),
    );
    this.rebuildProfileWatchers();
    this.refresh();
  }

  async initialize(): Promise<void> {
    await this.refreshDiscoveredProfiles();
    this.rebuildProfileWatchers();
    this.refresh();
  }

  activeFor(uri: vscode.Uri): ActiveEnvironmentProfile {
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    if (!folder) return { name: AUTO_PROFILE, files: [] };
    const config = vscode.workspace.getConfiguration("hurl", folder.uri);
    const profiles = this.profilesFor(folder);
    const defaultProfile = config.get<string>("environment.defaultProfile", AUTO_PROFILE);
    const selected = this.context.workspaceState.get<string>(`${STATE_PREFIX}${folder.uri.toString()}`);
    return resolveActiveProfile(folder.uri.toString(), selected ? { [folder.uri.toString()]: selected } : {}, defaultProfile, profiles);
  }

  descriptors(): Array<{ workspaceUri: string; name: string; files: string[] }> {
    return (vscode.workspace.workspaceFolders ?? []).map((folder) => ({
      workspaceUri: folder.uri.toString(),
      ...this.activeFor(folder.uri),
    }));
  }

  async selectForActiveEditor(): Promise<void> {
    const uri = vscode.window.activeTextEditor?.document.uri;
    const folder = uri && vscode.workspace.getWorkspaceFolder(uri);
    if (!uri || !folder) {
      void vscode.window.showWarningMessage("Open a Hurl file inside a workspace folder to select an environment.");
      return;
    }
    await this.refreshDiscoveredProfile(folder);
    const profiles = this.profilesFor(folder);
    const active = this.activeFor(uri);
    const choices = [
      { label: AUTO_PROFILE, description: "Automatically discover legacy Hurl variable files" },
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

  async createForActiveEditor(): Promise<void> {
    const folder = this.activeWorkspaceFolder();
    if (!folder) return;
    const active = vscode.window.activeTextEditor && this.activeFor(vscode.window.activeTextEditor.document.uri);
    const name = await vscode.window.showInputBox({
      title: "Create Hurl environment",
      prompt: "Environment name used in .env.<name>",
      value: active && active.name !== AUTO_PROFILE ? active.name : "",
      validateInput: (value) => /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(value)
        ? undefined
        : "Use letters, numbers, dots, underscores, or hyphens.",
    });
    if (!name) return;
    const picked = await vscode.window.showQuickPick([
      { label: `.env.${name}`, description: "Shared environment values" },
      { label: `.env.${name}.local`, description: "Local overrides (usually ignored by Git)" },
    ], { title: `Create environment file — ${folder.name}` });
    if (!picked) return;

    const uri = vscode.Uri.joinPath(folder.uri, picked.label);
    if (await this.fileExists(uri)) {
      void vscode.window.showWarningMessage(`${picked.label} already exists.`);
    } else {
      await vscode.workspace.fs.writeFile(uri, Buffer.from(""));
      await this.refreshDiscoveredProfile(folder);
      this.refresh();
      this.notifyAll();
    }
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri));
  }

  async openForActiveEditor(): Promise<void> {
    const folder = this.activeWorkspaceFolder();
    const editor = vscode.window.activeTextEditor;
    if (!folder || !editor) return;
    const active = this.activeFor(editor.document.uri);
    const candidates = active.name === AUTO_PROFILE
      ? [".hurl-vars", "vars.env", "hurl.env", ".env", ".env.local"]
      : active.files;
    const existing = [];
    for (const file of candidates) {
      if (await this.fileExists(vscode.Uri.joinPath(folder.uri, file))) existing.push(file);
    }
    if (!existing.length) {
      void vscode.window.showInformationMessage(`No files exist for the ${active.name} environment.`);
      return;
    }
    const picked = await vscode.window.showQuickPick(existing, {
      title: `Open ${active.name} environment files`,
      canPickMany: true,
      placeHolder: "Select one or more files",
    });
    for (const file of picked ?? []) {
      const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder.uri, file));
      await vscode.window.showTextDocument(document, { preview: false, preserveFocus: true });
    }
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

  private notifyAll(): void {
    for (const descriptor of this.descriptors()) {
      this.emitter.fire({ folderUri: descriptor.workspaceUri, profile: { name: descriptor.name, files: descriptor.files } });
    }
  }

  private rebuildProfileWatchers(): void {
    for (const disposable of this.profileWatchers) disposable.dispose();
    this.profileWatchers = [];
    if (!this.onVariableFileChange) return;

    for (const folder of vscode.workspace.workspaceFolders ?? []) {
      const config = vscode.workspace.getConfiguration("hurl", folder.uri);
      const conventionWatcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(folder, ".env*"));
      this.profileWatchers.push(
        conventionWatcher,
        conventionWatcher.onDidCreate((uri) => this.handleConventionFileChange(folder, uri, vscode.FileChangeType.Created)),
        conventionWatcher.onDidChange((uri) => this.onVariableFileChange?.(uri, vscode.FileChangeType.Changed)),
        conventionWatcher.onDidDelete((uri) => this.handleConventionFileChange(folder, uri, vscode.FileChangeType.Deleted)),
      );
      for (const file of profileWatchPaths(config.get("environment.profiles", {}))) {
        if (!file.includes("/") && file.startsWith(".env")) continue;
        const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(folder, file));
        this.profileWatchers.push(
          watcher,
          watcher.onDidCreate((uri) => this.onVariableFileChange?.(uri, vscode.FileChangeType.Created)),
          watcher.onDidChange((uri) => this.onVariableFileChange?.(uri, vscode.FileChangeType.Changed)),
          watcher.onDidDelete((uri) => this.onVariableFileChange?.(uri, vscode.FileChangeType.Deleted)),
        );
      }
    }
  }

  private profilesFor(folder: vscode.WorkspaceFolder): Record<string, string[]> {
    const config = vscode.workspace.getConfiguration("hurl", folder.uri);
    return mergeEnvironmentProfiles(
      this.discoveredProfiles.get(folder.uri.toString()) ?? {},
      normalizeProfiles(config.get("environment.profiles", {})),
    );
  }

  private activeWorkspaceFolder(): vscode.WorkspaceFolder | undefined {
    const uri = vscode.window.activeTextEditor?.document.uri;
    const folder = uri && vscode.workspace.getWorkspaceFolder(uri);
    if (!folder) void vscode.window.showWarningMessage("Open a Hurl file inside a workspace folder first.");
    return folder;
  }

  private async fileExists(uri: vscode.Uri): Promise<boolean> {
    try {
      const stat = await vscode.workspace.fs.stat(uri);
      return stat.type === vscode.FileType.File;
    } catch {
      return false;
    }
  }

  private async refreshDiscoveredProfiles(): Promise<void> {
    const folders = vscode.workspace.workspaceFolders ?? [];
    this.discoveredProfiles.clear();
    await Promise.all(folders.map((folder) => this.refreshDiscoveredProfile(folder)));
  }

  private async refreshDiscoveredProfile(folder: vscode.WorkspaceFolder): Promise<void> {
    const uris = await vscode.workspace.findFiles(new vscode.RelativePattern(folder, ".env*"));
    const files = uris.map((uri) => uri.path.slice(uri.path.lastIndexOf("/") + 1));
    this.discoveredProfiles.set(folder.uri.toString(), discoverConventionProfiles(files));
  }

  private handleConventionFileChange(
    folder: vscode.WorkspaceFolder,
    uri: vscode.Uri,
    type: vscode.FileChangeType,
  ): void {
    this.onVariableFileChange?.(uri, type);
    void this.refreshDiscoveredProfile(folder).then(() => {
      this.refresh();
      this.notifyAll();
    });
  }

  dispose(): void {
    for (const disposable of this.profileWatchers) disposable.dispose();
    for (const disposable of this.disposables) disposable.dispose();
  }
}
