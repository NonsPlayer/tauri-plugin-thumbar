export interface ThumbarButton {
    /** Unique identifier within the window, in 1..=65535. */
    id: number;
    /** Tauri resource path to an .ico file. */
    icon: string;
    tooltip?: string;
    enabled?: boolean;
    visible?: boolean;
    /** Optional app event emitted when the button is clicked. */
    event?: string;
}
export interface ThumbarButtonUpdate {
    id: number;
    icon?: string;
    tooltip?: string;
    enabled?: boolean;
    visible?: boolean;
}
export interface ThumbarClickEvent {
    id: number;
    window: string;
}
/**
 * Adds the complete thumbnail toolbar button list to the current window.
 *
 * Windows only allows buttons to be added once per window. Call `update` for
 * subsequent changes; at most 7 buttons are supported.
 */
export declare function initialize(buttons: ThumbarButton[]): Promise<void>;
/**
 * Updates an existing button's icon, tooltip, enabled or visible state.
 */
export declare function update(button: ThumbarButtonUpdate): Promise<void>;
/**
 * Sets the taskbar hover thumbnail from an absolute local image path.
 *
 * PNG, JPEG, WebP and GIF are supported, up to 16 MiB.
 */
export declare function setCoverFromPath(path: string): Promise<void>;
/**
 * Sets the taskbar hover thumbnail from encoded image bytes.
 *
 * PNG, JPEG, WebP and GIF are supported, up to 16 MiB.
 */
export declare function setCoverFromBytes(bytes: Uint8Array | ArrayBuffer): Promise<void>;
/**
 * Restores the live window thumbnail on the taskbar.
 */
export declare function clearCover(): Promise<void>;
/**
 * Returns whether the current platform supports native thumbnail toolbars.
 */
export declare function isSupported(): Promise<boolean>;
