export const CROP_MARKER_SIZE = 32;

export const IS_MAC = navigator.platform.toUpperCase().includes('MAC');
export const IS_IOS_SAFARI =
  typeof navigator !== 'undefined' &&
  /AppleWebKit/i.test(navigator.userAgent) &&
  !/(CriOS|FxiOS|EdgiOS)/i.test(navigator.userAgent) &&
  (/iPad|iPhone|iPod/i.test(navigator.userAgent) ||
    (navigator.userAgent.includes('Macintosh') && typeof document !== 'undefined' && 'ontouchend' in document));

export const CONTINUOUS_MIN_WIDTH = 300;
export const CONTINUOUS_VIEW_PADDING = 20;
export const CONTINUOUS_PAGE_MARGIN = 20;

export const PAGINATED_VIEW_PADDING = 0;
export const PAGE_GAP = 24;

export const TOUCH_LONG_PRESS_MS = 500;
export const TOUCH_DOUBLE_TAP_INTERVAL_MS = 300;
export const TOUCH_DOUBLE_TAP_DISTANCE_PX = 20;
export const TOUCH_DRAG_START_DISTANCE_PX = 4;
export const TOUCH_LONG_PRESS_CANCEL_DISTANCE_PX = 8;
export const TOUCH_EDGE_SCROLL_THRESHOLD_PX = 60;
export const TOUCH_EDGE_MIN_SCROLL_SPEED = 4;
export const TOUCH_EDGE_MAX_SCROLL_SPEED = 16;
