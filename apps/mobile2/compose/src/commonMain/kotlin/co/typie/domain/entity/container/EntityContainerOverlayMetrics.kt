package co.typie.domain.entity

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

internal fun initialEntityContainerBottomOverlayMetrics(
  baseBottomInset: Dp
): EntityContainerBottomOverlayMetrics {
  return calculateEntityContainerBottomOverlayMetrics(
    baseBottomInset = baseBottomInset,
    hasPasteBar = false,
    pasteBarHeight = EntityBottomOverlayDefaults.BarHeight,
    hasSelectionBar = false,
    selectionBarHeight = EntityBottomOverlayDefaults.BarHeight,
  )
}

internal fun calculateEntityContainerBottomOverlayMetrics(
  baseBottomInset: Dp,
  hasPasteBar: Boolean,
  pasteBarHeight: Dp,
  hasSelectionBar: Boolean,
  selectionBarHeight: Dp,
): EntityContainerBottomOverlayMetrics {
  val visibleHeights = buildList {
    if (hasSelectionBar) add(selectionBarHeight)
    if (hasPasteBar) add(pasteBarHeight)
  }
  if (visibleHeights.isEmpty()) {
    return EntityContainerBottomOverlayMetrics(
      occupiedHeight = 0.dp,
      reservedSpacerHeight = EntityBottomOverlayDefaults.DefaultBottomSpacerHeight,
    )
  }

  val stackHeight =
    visibleHeights.fold(0.dp) { total, height -> total + height } +
      EntityBottomOverlayDefaults.Gap * (visibleHeights.size - 1)
  val occupiedHeight = baseBottomInset + EntityBottomOverlayDefaults.BottomOffset + stackHeight

  return EntityContainerBottomOverlayMetrics(
    occupiedHeight = occupiedHeight,
    reservedSpacerHeight =
      maxOf(
        EntityBottomOverlayDefaults.DefaultBottomSpacerHeight,
        occupiedHeight + EntityBottomOverlayDefaults.ReserveExtra,
      ),
  )
}
