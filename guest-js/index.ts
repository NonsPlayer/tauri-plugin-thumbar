import { invoke } from '@tauri-apps/api/core'

export interface ThumbarButton {
  /** Unique identifier within the window, in 1..=65535. */
  id: number
  /** Tauri resource path to an .ico file. */
  icon: string
  tooltip?: string
  enabled?: boolean
  visible?: boolean
  /** Optional app event emitted when the button is clicked. */
  event?: string
}

export interface ThumbarButtonUpdate {
  id: number
  icon?: string
  tooltip?: string
  enabled?: boolean
  visible?: boolean
}

export interface ThumbarClickEvent {
  id: number
  window: string
}

/**
 * Adds the complete thumbnail toolbar button list to the current window.
 *
 * Windows only allows buttons to be added once per window. Call `update` for
 * subsequent changes; at most 7 buttons are supported.
 */
export async function initialize(buttons: ThumbarButton[]): Promise<void> {
  await invoke('plugin:thumbar|initialize', { buttons })
}

/**
 * Updates an existing button's icon, tooltip, enabled or visible state.
 */
export async function update(button: ThumbarButtonUpdate): Promise<void> {
  await invoke('plugin:thumbar|update', { button })
}

/**
 * Returns whether the current platform supports native thumbnail toolbars.
 */
export async function isSupported(): Promise<boolean> {
  return await invoke<boolean>('plugin:thumbar|is_supported')
}