import { invoke } from '@tauri-apps/api/core';

/** Mirrors `commands::AppInfo` in `src-tauri/src/commands.rs`. */
export interface AppInfo {
  name: string;
  version: string;
  tauri_version: string;
  rebase_release: number | null;
  enzyme_count: number;
}

export const appInfo = (): Promise<AppInfo> => invoke<AppInfo>('app_info');

/** Window ▸ Show/Hide Help Window. Resolves to the new visibility. */
export const toggleHelpWindow = (): Promise<boolean> => invoke<boolean>('toggle_help_window');
