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
 * Sets the taskbar hover thumbnail from an absolute local image path.
 *
 * PNG, JPEG, WebP and GIF are supported, up to 16 MiB.
 */
export async function setCoverFromPath(path: string): Promise<void> {
  await invoke('plugin:thumbar|set_cover', { cover: { path } })
}

/**
 * Sets the taskbar hover thumbnail from encoded image bytes.
 *
 * PNG, JPEG, WebP and GIF are supported, up to 16 MiB.
 */
export async function setCoverFromBytes(bytes: Uint8Array | ArrayBuffer): Promise<void> {
  const data = toBase64(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes))
  await invoke('plugin:thumbar|set_cover', { cover: { data } })
}

/**
 * Restores the live window thumbnail on the taskbar.
 */
export async function clearCover(): Promise<void> {
  await invoke('plugin:thumbar|set_cover', { cover: null })
}

/**
 * Returns whether the current platform supports native thumbnail toolbars.
 */
export async function isSupported(): Promise<boolean> {
  return await invoke<boolean>('plugin:thumbar|is_supported')
}

function toBase64(bytes: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000

  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize))
  }

  return btoa(binary)
}