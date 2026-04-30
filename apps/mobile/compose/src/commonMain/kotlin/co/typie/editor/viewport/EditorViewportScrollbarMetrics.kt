package co.typie.editor.viewport

internal data class EditorViewportScrollbarMetrics(
  val isVisible: Boolean,
  val trackLength: Float,
  val thumbSize: Float,
  val thumbOffset: Float,
  val thumbTravel: Float,
)

internal fun shouldShowEditorViewportScrollbarThumb(
  viewportLength: Float,
  contentLength: Float,
  trackLength: Float = resolveEditorViewportScrollbarTrackLength(viewportLength),
  minThumbSize: Float = 0f,
): Boolean {
  val effectiveMinThumbSize = minThumbSize.coerceAtLeast(0f)
  val hasUsableTrack =
    if (effectiveMinThumbSize > 0f) {
      trackLength >= effectiveMinThumbSize
    } else {
      trackLength > 0f
    }

  return viewportLength > 0f && contentLength > viewportLength && hasUsableTrack
}

internal fun resolveEditorViewportScrollbarTrackLength(
  viewportLength: Float,
  leadingInset: Float = 0f,
  trailingInset: Float = 0f,
  leadingPadding: Float = 0f,
  trailingPadding: Float = 0f,
): Float =
  (viewportLength - leadingInset - trailingInset - leadingPadding - trailingPadding).coerceAtLeast(
    0f
  )

internal fun resolveEditorViewportScrollbarThumbSize(
  trackLength: Float,
  viewportLength: Float,
  contentLength: Float,
  minThumbSize: Float,
): Float {
  val effectiveMinThumbSize = minThumbSize.coerceAtLeast(0f)
  if (
    trackLength <= 0f ||
      viewportLength <= 0f ||
      contentLength <= viewportLength ||
      trackLength < effectiveMinThumbSize
  ) {
    return 0f
  }

  val rawThumbSize = trackLength * viewportLength / contentLength
  return rawThumbSize.coerceIn(effectiveMinThumbSize, trackLength)
}

internal fun resolveEditorViewportScrollbarThumbOffset(
  trackLength: Float,
  thumbSize: Float,
  viewportLength: Float,
  contentLength: Float,
  scrollPosition: Float,
): Float {
  val maxScroll = (contentLength - viewportLength).coerceAtLeast(0f)
  val thumbTravel = (trackLength - thumbSize).coerceAtLeast(0f)
  if (maxScroll <= 0f || thumbTravel <= 0f) {
    return 0f
  }

  val scrollRatio = (scrollPosition / maxScroll).coerceIn(0f, 1f)
  return thumbTravel * scrollRatio
}

internal fun resolveEditorViewportScrollbarScrollPositionFromDrag(
  startScrollPosition: Float,
  dragDelta: Float,
  trackLength: Float,
  thumbSize: Float,
  viewportLength: Float,
  contentLength: Float,
): Float {
  val maxScroll = (contentLength - viewportLength).coerceAtLeast(0f)
  val thumbTravel = (trackLength - thumbSize).coerceAtLeast(0f)
  if (maxScroll <= 0f || thumbTravel <= 0f) {
    return startScrollPosition.coerceIn(0f, maxScroll)
  }

  val scrollDelta = dragDelta * maxScroll / thumbTravel
  return (startScrollPosition + scrollDelta).coerceIn(0f, maxScroll)
}

internal fun resolveEditorViewportScrollbarMetrics(
  viewportLength: Float,
  contentLength: Float,
  scrollPosition: Float,
  minThumbSize: Float,
  leadingInset: Float = 0f,
  trailingInset: Float = 0f,
  leadingPadding: Float = 0f,
  trailingPadding: Float = 0f,
): EditorViewportScrollbarMetrics {
  val trackLength =
    resolveEditorViewportScrollbarTrackLength(
      viewportLength = viewportLength,
      leadingInset = leadingInset,
      trailingInset = trailingInset,
      leadingPadding = leadingPadding,
      trailingPadding = trailingPadding,
    )
  val isVisible =
    shouldShowEditorViewportScrollbarThumb(
      viewportLength = viewportLength,
      contentLength = contentLength,
      trackLength = trackLength,
      minThumbSize = minThumbSize,
    )
  val thumbSize =
    if (isVisible) {
      resolveEditorViewportScrollbarThumbSize(
        trackLength = trackLength,
        viewportLength = viewportLength,
        contentLength = contentLength,
        minThumbSize = minThumbSize,
      )
    } else {
      0f
    }
  val thumbOffset =
    if (isVisible) {
      resolveEditorViewportScrollbarThumbOffset(
        trackLength = trackLength,
        thumbSize = thumbSize,
        viewportLength = viewportLength,
        contentLength = contentLength,
        scrollPosition = scrollPosition,
      )
    } else {
      0f
    }
  val thumbTravel = (trackLength - thumbSize).coerceAtLeast(0f)

  return EditorViewportScrollbarMetrics(
    isVisible = isVisible,
    trackLength = trackLength,
    thumbSize = thumbSize,
    thumbOffset = thumbOffset,
    thumbTravel = thumbTravel,
  )
}
