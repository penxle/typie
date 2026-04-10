package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.abs

object SheetDetentResolver {
  fun resolve(
    policy: SheetSizePolicy,
    context: SheetDetentContext,
  ): List<ResolvedSheetDetent> =
    when (policy) {
      SheetSizePolicy.Intrinsic -> listOf(resolveDetent(SheetDetent.Intrinsic, context))
      is SheetSizePolicy.Fixed -> listOf(resolveDetent(SheetDetent.Fixed(policy.height), context))
      is SheetSizePolicy.Max -> listOf(resolveDetent(SheetDetent.TopGap(policy.topGap), context))
      is SheetSizePolicy.Detents -> (listOf(policy.initial) + policy.available)
        .map { resolveDetent(it, context) }
        .distinctBy { it.id }
        .sortedBy { it.height.value }
    }

  fun resolveDetent(
    detent: SheetDetent,
    context: SheetDetentContext,
  ): ResolvedSheetDetent {
    val height = when (detent) {
      SheetDetent.Intrinsic -> context.contentHeight
      is SheetDetent.Fixed -> detent.height
      is SheetDetent.Fraction -> context.viewportHeight * detent.value
      is SheetDetent.TopGap -> context.viewportHeight - detent.gap
      is SheetDetent.Content -> {
        val maxHeight = detent.maxTopGap?.let { context.viewportHeight - it } ?: context.viewportHeight
        minOf(context.contentHeight, maxHeight)
      }

      is SheetDetent.Custom -> detent.resolver(context)
    }.coerceIn(0.dp, context.viewportHeight)

    return ResolvedSheetDetent(
      id = detent.id,
      height = height,
    )
  }
}

internal fun resolveSheetSettledDetent(
  policy: SheetSizePolicy,
  detents: List<ResolvedSheetDetent>,
  currentDetentId: SheetDetentId?,
  sheetHeight: Dp,
  velocity: Float,
): ResolvedSheetDetent? {
  if (detents.isEmpty()) {
    return null
  }

  val nearest = detents.minByOrNull { detent ->
    abs((detent.height - sheetHeight).value)
  } ?: return null
  val current = detents.firstOrNull { it.id == currentDetentId } ?: return nearest
  val largerDetents = detents.filter { it.height > current.height }
  val smallerDetents = detents.filter { it.height < current.height }

  if (
    policy.allowsDragExpansion() &&
    largerDetents.isNotEmpty() &&
    (
      velocity <= -SheetDefaults.DetentSnapVelocityThreshold ||
      sheetHeight >= current.height + minOf(
        SheetDefaults.DetentSnapThreshold,
        largerDetents.first().height - current.height,
      )
    )
  ) {
    return largerDetents.minByOrNull { detent ->
      abs((detent.height - sheetHeight).value)
    } ?: current
  }

  if (
    policy.allowsDragCollapse() &&
    smallerDetents.isNotEmpty() &&
    (
      velocity >= SheetDefaults.DetentSnapVelocityThreshold ||
      sheetHeight <= current.height - minOf(
        SheetDefaults.DetentSnapThreshold,
        current.height - smallerDetents.last().height,
      )
    )
  ) {
    return smallerDetents.minByOrNull { detent ->
      abs((detent.height - sheetHeight).value)
    } ?: current
  }

  return current
}

internal fun shouldDismissDraggedSheet(
  policy: SheetSizePolicy,
  detents: List<ResolvedSheetDetent>,
  currentDetentId: SheetDetentId?,
  sheetHeight: Dp,
  velocity: Float,
): Boolean {
  val minDetent = detents.minByOrNull { it.height.value } ?: return false
  val currentDetent = detents.firstOrNull { it.id == currentDetentId } ?: minDetent
  val dismissAnchor = when (policy) {
    is SheetSizePolicy.Detents -> when (policy.dragDismissBehavior) {
      SheetDragDismissBehavior.FromMinDetent -> minDetent
      SheetDragDismissBehavior.FromCurrentDetent -> currentDetent
    }

    else -> minDetent
  }
  val dismissThreshold = dismissAnchor.height * (1f - SheetDefaults.DismissThresholdFraction)

  return sheetHeight < dismissThreshold ||
    (velocity > SheetDefaults.DismissVelocityThreshold && sheetHeight <= dismissAnchor.height)
}
