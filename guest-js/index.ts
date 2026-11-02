import { invoke } from "@tauri-apps/api/core";

export interface TaskbarEventNames {
  previous?: string;
  toggle?: string;
  next?: string;
}

export interface TaskbarTooltips {
  previous?: string;
  play?: string;
  pause?: string;
  next?: string;
}

export interface TaskbarIconPaths {
  previous?: string;
  previousDisabled?: string;
  play?: string;
  pause?: string;
  next?: string;
  nextDisabled?: string;
}

export interface TaskbarPluginConfig {
  autoAttach?: boolean;
  windowLabel?: string;
  events?: TaskbarEventNames;
  tooltips?: TaskbarTooltips;
  icons?: TaskbarIconPaths;
}

/**
 * Explicitly attach taskbar thumbnail buttons to the current window.
 * Usually optional when autoAttach is enabled in plugin config.
 */
export async function initialize(): Promise<void> {
  await invoke("plugin:taskbar|initialize");
}

/**
 * Updates the play/pause thumbnail button icon and tooltip.
 */
export async function setPlaybackState(isPlaying: boolean): Promise<void> {
  await invoke("plugin:taskbar|set_playback_state", { isPlaying });
}

/**
 * Updates enabled state + icon state for previous/next thumbnail buttons.
 */
export async function setNavigationEnabled(
  previousEnabled: boolean,
  nextEnabled: boolean,
): Promise<void> {
  await invoke("plugin:taskbar|set_navigation_enabled", {
    previousEnabled,
    nextEnabled,
  });
}

/**
 * Returns whether the current platform supports this plugin's native behavior.
 */
export async function isSupported(): Promise<boolean> {
  return await invoke<boolean>("plugin:taskbar|is_supported");
}
