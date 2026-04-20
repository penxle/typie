package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Dp

internal const val SheetAnchorTolerancePx = 0.5f

sealed interface SheetStop {
  enum class Policy {
    KeepAll,
    DismissFromTopStop,
  }

  data class Bottom(val height: Dp) : SheetStop

  data class Top(val margin: Dp) : SheetStop
}

internal data class SheetAnchor(val value: Int, val offset: Float)

internal fun hasSheetReachedTopStop(
  currentOffset: Float,
  topStopOffset: Float,
  tolerancePx: Float = SheetAnchorTolerancePx,
): Boolean {
  if (currentOffset.isNaN()) {
    return false
  }

  return currentOffset <= topStopOffset + tolerancePx
}

internal fun resolveEffectiveSheetAnchors(
  anchors: List<SheetAnchor>,
  stopPolicy: SheetStop.Policy,
  hasReachedTopStop: Boolean,
  tolerancePx: Float = SheetAnchorTolerancePx,
): List<SheetAnchor> {
  if (
    stopPolicy != SheetStop.Policy.DismissFromTopStop || !hasReachedTopStop || anchors.isEmpty()
  ) {
    return anchors
  }

  val topOffset = anchors.minOf(SheetAnchor::offset)
  return anchors.filter { anchor -> anchor.offset <= topOffset + tolerancePx }
}
