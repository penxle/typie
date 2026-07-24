const EDITOR_HEADER_MIN_WIDTH = 320;
const EDITOR_HEADER_MIN_SCALE = 0.75;
const EDITOR_HEADER_VIEWPORT_GAP = 20;

type HeaderGeometryInput = {
  viewportWidth: number;
  displayZoom: number;
  bodyTrackWidth: number;
  contentInsetLeft: number;
  contentInsetRight: number;
};

type HeaderGeometry = {
  trackWidth: number;
  contentInsetLeft: number;
  contentInsetRight: number;
  fieldWidth: number;
  stickyLeft: number;
};

const nonNegativeFinite = (value: number): number => (Number.isFinite(value) ? Math.max(0, value) : 0);

export function resolveHeaderGeometry(input: HeaderGeometryInput): HeaderGeometry | null {
  if (
    !Number.isFinite(input.viewportWidth) ||
    input.viewportWidth <= 0 ||
    !Number.isFinite(input.displayZoom) ||
    input.displayZoom <= 0 ||
    !Number.isFinite(input.bodyTrackWidth) ||
    input.bodyTrackWidth <= 0
  ) {
    return null;
  }

  const viewportWidth = input.viewportWidth;
  const displayZoom = input.displayZoom;
  const bodyTrackWidth = input.bodyTrackWidth;
  const leftInset = nonNegativeFinite(input.contentInsetLeft * displayZoom);
  const rightInset = nonNegativeFinite(input.contentInsetRight * displayZoom);
  const readableMin = Math.max(EDITOR_HEADER_MIN_WIDTH * displayZoom, EDITOR_HEADER_MIN_WIDTH * EDITOR_HEADER_MIN_SCALE);
  const visibleCapacity = Math.max(0, viewportWidth - leftInset - rightInset);
  const trackWidth = Math.max(bodyTrackWidth, leftInset + rightInset + Math.min(readableMin, visibleCapacity));
  const fieldWidth = Math.min(
    Math.max(0, trackWidth - leftInset - rightInset),
    Math.max(0, viewportWidth - EDITOR_HEADER_VIEWPORT_GAP * 2),
  );

  return {
    trackWidth,
    contentInsetLeft: leftInset,
    contentInsetRight: rightInset,
    fieldWidth,
    stickyLeft: EDITOR_HEADER_VIEWPORT_GAP,
  };
}
