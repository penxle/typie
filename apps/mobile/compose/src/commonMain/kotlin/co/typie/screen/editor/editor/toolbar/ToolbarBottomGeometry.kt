package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

internal fun resolveEditorToolbarBottomPanelLayoutHeight(
  bottomPanelVisible: Boolean,
  bottomPanelHeight: Dp,
): Dp = if (bottomPanelVisible) ToolbarBottomPanelGap + bottomPanelHeight else 0.dp

internal fun shouldAnimateEditorToolbarBottomPanelLayoutHeightChange(
  bottomPanelVisible: Boolean,
  softwareKeyboardVisible: Boolean,
  previousBottomPanelLayoutHeight: Dp,
  bottomPanelLayoutHeight: Dp,
): Boolean =
  bottomPanelVisible &&
    !softwareKeyboardVisible &&
    previousBottomPanelLayoutHeight != bottomPanelLayoutHeight

internal fun resolveEditorToolbarBottomSpacerHeight(
  bottomPanelVisible: Boolean,
  bottomPanelLayoutHeight: Dp,
  inputBottomInset: Dp,
  safeBottomInset: Dp,
): Dp =
  if (bottomPanelVisible) {
    bottomPanelLayoutHeight + safeBottomInset
  } else {
    inputBottomInset
  }

internal fun resolveEditorToolbarBottomInset(
  bottomSpacerHeight: Dp,
  bottomPanelLayoutHeight: Dp,
  safeBottomInset: Dp,
): Dp =
  (maxOf(bottomSpacerHeight, bottomPanelLayoutHeight + safeBottomInset) - bottomPanelLayoutHeight)
    .coerceAtLeast(0.dp)
