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
