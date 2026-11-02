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
export declare function initialize(): Promise<void>;
/**
 * Updates the play/pause thumbnail button icon and tooltip.
 */
export declare function setPlaybackState(isPlaying: boolean): Promise<void>;
/**
 * Updates enabled state + icon state for previous/next thumbnail buttons.
 */
export declare function setNavigationEnabled(previousEnabled: boolean, nextEnabled: boolean): Promise<void>;
/**
 * Returns whether the current platform supports this plugin's native behavior.
 */
export declare function isSupported(): Promise<boolean>;
