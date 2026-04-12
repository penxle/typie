package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Density

internal fun resolveInitialSheetHeightPx(
  density: Density,
  requiresContentMeasurement: Boolean,
  initialResolvedDetent: ResolvedSheetDetent?,
): Float =
  when {
    requiresContentMeasurement -> 0f
    initialResolvedDetent != null -> with(density) { initialResolvedDetent.height.toPx() }
    else -> 0f
  }

internal fun resolveRenderedSheetHeightPx(
  currentSheetHeightPx: Float,
  requiresContentMeasurement: Boolean,
  measuredSheetHeightPx: Float,
  initialSheetHeightPx: Float,
  targetSheetHeightPx: Float? = null,
  shouldAnimateTargetHeightChange: Boolean = true,
): Float =
  when {
    targetSheetHeightPx != null && targetSheetHeightPx > 0f && !shouldAnimateTargetHeightChange ->
      targetSheetHeightPx
    currentSheetHeightPx > 0f -> currentSheetHeightPx
    requiresContentMeasurement -> measuredSheetHeightPx
    else -> initialSheetHeightPx
  }

internal fun shouldRequestModalSheetFocus(
  isTopOfStack: Boolean,
  mode: SheetMode,
  progress: Float,
  sheetHasFocus: Boolean,
): Boolean = isTopOfStack && mode == SheetMode.Modal && progress > 0f && !sheetHasFocus

internal fun resolveSheetSurfaceAlpha(
  requiresContentMeasurement: Boolean,
  isContentReady: Boolean,
): Float =
  if (requiresContentMeasurement && !isContentReady) {
    0f
  } else {
    1f
  }

internal fun shouldTreatMeasuredSheetAsConstrained(
  requiresContentMeasurement: Boolean,
  measuredSheetHeightPx: Float,
  renderedSheetHeightPx: Float,
): Boolean {
  if (!requiresContentMeasurement || measuredSheetHeightPx <= 0f) {
    return false
  }
  return renderedSheetHeightPx + 0.5f < measuredSheetHeightPx
}

internal fun shouldApplyRenderedSheetHeight(
  policy: SheetSizePolicy,
  targetDetentId: SheetDetentId?,
  requiresContentMeasurement: Boolean,
  measuredSheetHeightPx: Float,
  renderedSheetHeightPx: Float,
): Boolean {
  if (renderedSheetHeightPx <= 0f) {
    return false
  }

  if (!isContentDrivenSheetDetent(policy = policy, targetDetentId = targetDetentId)) {
    return true
  }

  return shouldTreatMeasuredSheetAsConstrained(
    requiresContentMeasurement = requiresContentMeasurement,
    measuredSheetHeightPx = measuredSheetHeightPx,
    renderedSheetHeightPx = renderedSheetHeightPx,
  )
}

internal fun isContentDrivenSheetDetent(
  policy: SheetSizePolicy,
  targetDetentId: SheetDetentId?,
): Boolean =
  when (policy) {
    is SheetSizePolicy.Intrinsic -> true
    is SheetSizePolicy.Fixed,
    is SheetSizePolicy.Max -> false

    is SheetSizePolicy.Detents -> {
      val detentId = targetDetentId ?: policy.initial.id
      (listOf(policy.initial) + policy.available)
        .firstOrNull { it.id == detentId }
        ?.requiresContentMeasurement() == true
    }
  }

internal fun shouldAnimateSheetHeightChange(
  policy: SheetSizePolicy,
  targetDetentId: SheetDetentId?,
  lastSettledDetentId: SheetDetentId?,
): Boolean =
  !isContentDrivenSheetDetent(policy = policy, targetDetentId = targetDetentId) ||
    targetDetentId != lastSettledDetentId
