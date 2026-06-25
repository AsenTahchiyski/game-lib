// Thin wrappers around Tauri commands and native dialogs.
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile, stat } from "@tauri-apps/plugin-fs";
import type { Library, Settings } from "./types";
import type { SteamGame, GogSyncResult, EpicSyncResult, IgnGame } from "./sync";

const JSON_FILTER = [{ name: "Game Library", extensions: ["json"] }];

/** Native "open file" picker. Returns the chosen path, or null if cancelled. */
export async function pickOpenPath(): Promise<string | null> {
  const result = await open({ multiple: false, directory: false, filters: JSON_FILTER });
  return typeof result === "string" ? result : null;
}

/** Native "save file" picker. Returns the chosen path, or null if cancelled. */
export async function pickSavePath(): Promise<string | null> {
  const result = await save({ defaultPath: "game-library.json", filters: JSON_FILTER });
  return result ?? null;
}

// File IO lives in JS via plugin-fs (not a Rust command) because on Android the
// dialog hands back a `content://` URI that std::fs can't open — plugin-fs
// routes those through the platform's ContentResolver.

export async function readLibrary(path: string): Promise<Library> {
  const data = await readTextFile(path);
  try {
    return JSON.parse(data) as Library;
  } catch (e) {
    throw new Error(`Invalid library JSON in ${path}: ${e}`);
  }
}

export const writeLibrary = (path: string, library: Library) =>
  writeTextFile(path, JSON.stringify(library, null, 2));

export async function fileMtime(path: string): Promise<number | null> {
  try {
    const info = await stat(path);
    return info.mtime ? info.mtime.getTime() : null;
  } catch {
    // File doesn't exist (or unreadable) — treat as "no copy on disk".
    return null;
  }
}

export const loadSettings = () => invoke<Settings>("load_settings");

export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });

export const syncSteam = (apiKey: string, steamId: string) =>
  invoke<SteamGame[]>("sync_steam", { apiKey, steamId });

export const gogLoginUrl = () => invoke<string>("gog_login_url");

export const gogExchangeCode = (code: string) =>
  invoke<string>("gog_exchange_code", { code });

export const gogSync = (refreshToken: string) =>
  invoke<GogSyncResult>("gog_sync", { refreshToken });

export const epicLoginUrl = () => invoke<string>("epic_login_url");

export const epicExchangeCode = (code: string) =>
  invoke<string>("epic_exchange_code", { code });

export const epicSync = (refreshToken: string) =>
  invoke<EpicSyncResult>("epic_sync", { refreshToken });

export const ignSync = (nickname: string) =>
  invoke<IgnGame[]>("ign_sync", { nickname });
