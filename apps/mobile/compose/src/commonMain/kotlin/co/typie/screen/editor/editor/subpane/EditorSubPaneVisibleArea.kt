package co.typie.screen.editor.editor.subpane

internal fun resolveSubPaneBottomOcclusion(layoutInfo: EditorSubPaneLayoutInfo?): Float =
  when (layoutInfo?.visibleAreaMode) {
    EditorSubPaneVisibleAreaMode.ResizeEditor -> layoutInfo.visibleHeight.coerceAtLeast(0f)
    EditorSubPaneVisibleAreaMode.OverlayEditor,
    null -> 0f
  }

internal fun resolveRelatedNotesVisibleAreaMode(
  sheetHeight: Float,
  expandedHeight: Float,
  tolerance: Float = 0.5f,
): EditorSubPaneVisibleAreaMode =
  if (expandedHeight > 0f && sheetHeight >= expandedHeight - tolerance) {
    EditorSubPaneVisibleAreaMode.OverlayEditor
  } else {
    EditorSubPaneVisibleAreaMode.ResizeEditor
  }
