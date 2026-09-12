export const AUTO_PROFILE = "Auto";
export type EnvironmentProfiles = Record<string, string[]>;
export type ActiveEnvironmentProfile = { name: string; files: string[] };

export function normalizeProfiles(value: unknown): EnvironmentProfiles {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const profiles: EnvironmentProfiles = {};
  for (const [name, files] of Object.entries(value)) {
    if (!name.trim() || !Array.isArray(files) || !files.every((file) => typeof file === "string" && file.trim())) continue;
    profiles[name] = files.map((file) => file.trim());
  }
  return profiles;
}

export function profileWatchPaths(value: unknown): string[] {
  const paths = Object.values(normalizeProfiles(value)).flat();
  return [...new Set(paths.filter(isWorkspaceRelativePath))];
}

export function discoverConventionProfiles(files: readonly string[]): EnvironmentProfiles {
  const available = new Set(files);
  const names = new Set<string>();
  for (const file of available) {
    if (!file.startsWith(".env.")) continue;
    const suffix = file.slice(".env.".length);
    const name = suffix.endsWith(".local") ? suffix.slice(0, -".local".length) : suffix;
    if (name && name !== "local") names.add(name);
  }

  const profiles: EnvironmentProfiles = {};
  for (const name of [...names].sort()) {
    profiles[name] = [".env", ".env.local", `.env.${name}`, `.env.${name}.local`]
      .filter((file) => available.has(file));
  }
  return profiles;
}

export function mergeEnvironmentProfiles(
  discovered: EnvironmentProfiles,
  configured: EnvironmentProfiles,
): EnvironmentProfiles {
  return { ...discovered, ...configured };
}

function isWorkspaceRelativePath(value: string): boolean {
  const normalized = value.replace(/\\/g, "/");
  if (normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) return false;
  return !normalized.split("/").includes("..");
}

export function resolveActiveProfile(
  folderUri: string,
  selections: Readonly<Record<string, string>>,
  defaultProfile: string,
  profiles: EnvironmentProfiles,
): ActiveEnvironmentProfile {
  const selected = selections[folderUri] || defaultProfile || AUTO_PROFILE;
  if (selected === AUTO_PROFILE) return { name: AUTO_PROFILE, files: [] };
  const files = profiles[selected];
  return files ? { name: selected, files: [...files] } : { name: AUTO_PROFILE, files: [] };
}
