package co.typie.ui.component.sheet

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.safeDrawing
import co.typie.ext.safeDrawingHorizontalPadding
import co.typie.ext.thenIf
import co.typie.ui.theme.AppShapes
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.roundToInt
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

private const val AnchorHidden = -1
private const val AnchorVisible = 0
private val DefaultIntrinsicTopGap = 64.dp
internal val SheetAnimationSpec = spring<Float>(stiffness = 500f)

internal interface AnchoredSheetSurfaceScope {
  fun dismiss()
}

internal data class AnchoredSheetGeometry(val visibleHeight: Float)

internal fun resolveAnchoredSheetVisibleHeight(
  containerHeightPx: Float,
  sheetOffsetPx: Float,
  density: Float,
): Float {
  val safeDensity = density.takeIf { it > 0f } ?: 1f
  return (containerHeightPx - sheetOffsetPx).coerceIn(0f, containerHeightPx) / safeDensity
}

@Composable
internal fun AnchoredSheetSurface(
  stops: List<SheetStop>,
  stopPolicy: SheetStop.Policy,
  modifier: Modifier = Modifier,
  onDismissed: () -> Unit,
  onDismissStarted: () -> Unit = {},
  onDismissCancelled: () -> Unit = {},
  onGeometryChanged: (AnchoredSheetGeometry) -> Unit = {},
  onSettledStopChanged: (Int?) -> Unit = {},
  scrim: @Composable AnchoredSheetSurfaceScope.(alpha: Float) -> Unit = {},
  content: @Composable AnchoredSheetSurfaceScope.() -> Unit,
) {
  BoxWithConstraints(modifier.fillMaxSize()) {
    val density = LocalDensity.current
    val containerHeightPx = with(density) { maxHeight.toPx() }
    val intrinsicTopLimitPx =
      with(density) {
        maxOf(WindowInsets.safeDrawing.getTop(density).toFloat(), DefaultIntrinsicTopGap.toPx())
      }
    val isIntrinsic = stops.isEmpty()
    var contentHeightPx by remember { mutableStateOf(0f) }
    var hasReachedTopStop by remember(stops, stopPolicy) { mutableStateOf(false) }
    var hasSettledVisible by remember { mutableStateOf(false) }
    var dismissing by remember { mutableStateOf(false) }
    var dismissed by remember { mutableStateOf(false) }
    var sheetHasFocus by remember { mutableStateOf(false) }
    val coroutineScope = rememberCoroutineScope()
    val focusManager = LocalFocusManager.current
    val dragOverscrollEffect = remember { SheetTopHysteresisOverscrollEffect() }

    val baseVisibleAnchors =
      remember(stops, containerHeightPx, contentHeightPx, intrinsicTopLimitPx) {
        if (isIntrinsic) {
          if (contentHeightPx > 0f) {
            listOf(
              SheetAnchor(
                value = AnchorVisible,
                offset = maxOf(containerHeightPx - contentHeightPx, intrinsicTopLimitPx),
              )
            )
          } else {
            emptyList()
          }
        } else {
          stops.mapIndexed { index, stop ->
            SheetAnchor(
              value = index,
              offset =
                when (stop) {
                  is SheetStop.Bottom -> containerHeightPx - with(density) { stop.height.toPx() }
                  is SheetStop.Top -> with(density) { stop.margin.toPx() }
                },
            )
          }
        }
      }
    val topVisibleOffset = baseVisibleAnchors.minOfOrNull(SheetAnchor::offset) ?: containerHeightPx
    val visibleAnchors =
      remember(baseVisibleAnchors, stopPolicy, hasReachedTopStop) {
        resolveEffectiveSheetAnchors(
          anchors = baseVisibleAnchors,
          stopPolicy = stopPolicy,
          hasReachedTopStop = hasReachedTopStop,
        )
      }

    val anchors =
      remember(visibleAnchors, containerHeightPx) {
        DraggableAnchors {
          visibleAnchors.forEach { anchor -> anchor.value at anchor.offset }
          AnchorHidden at containerHeightPx
        }
      }

    val anchoredState = remember {
      AnchoredDraggableState(initialValue = AnchorHidden, anchors = anchors)
    }

    LaunchedEffect(stopPolicy, isIntrinsic, topVisibleOffset, baseVisibleAnchors, anchoredState) {
      if (
        isIntrinsic ||
          stopPolicy != SheetStop.Policy.DismissFromTopStop ||
          baseVisibleAnchors.isEmpty()
      ) {
        return@LaunchedEffect
      }

      snapshotFlow { anchoredState.offset }
        .collect { currentOffset ->
          if (!hasReachedTopStop && hasSheetReachedTopStop(currentOffset, topVisibleOffset)) {
            hasReachedTopStop = true
          }
        }
    }

    val offsetCorrection = remember { Animatable(0f) }

    if (isIntrinsic) {
      remember(anchors) { anchoredState.updateAnchors(anchors, anchoredState.targetValue) }
    } else {
      LaunchedEffect(anchors) {
        val prevOffset = anchoredState.offset
        anchoredState.updateAnchors(anchors, anchoredState.targetValue)
        val newOffset = anchoredState.offset

        if (
          !prevOffset.isNaN() &&
            !newOffset.isNaN() &&
            prevOffset != newOffset &&
            anchoredState.currentValue != AnchorHidden
        ) {
          offsetCorrection.snapTo(prevOffset - newOffset)
          offsetCorrection.animateTo(0f, SheetAnimationSpec)
        }
      }
    }

    LaunchedEffect(baseVisibleAnchors.isNotEmpty()) {
      if (baseVisibleAnchors.isEmpty()) return@LaunchedEffect

      anchoredState.animateTo(AnchorVisible, SheetAnimationSpec)
    }

    val hapticFeedback = LocalHapticFeedback.current
    val hapticFeedbackState = rememberUpdatedState(hapticFeedback)
    val onDismissedState = rememberUpdatedState(onDismissed)
    val onDismissStartedState = rememberUpdatedState(onDismissStarted)
    val onDismissCancelledState = rememberUpdatedState(onDismissCancelled)
    val onGeometryChangedState = rememberUpdatedState(onGeometryChanged)
    val onSettledStopChangedState = rememberUpdatedState(onSettledStopChanged)

    fun finishDismiss() {
      if (dismissed) {
        return
      }

      dismissed = true
      onDismissedState.value()
    }

    fun beginDismiss() {
      if (dismissing || dismissed) {
        return
      }

      dismissing = true
      if (sheetHasFocus) {
        focusManager.clearFocus()
      }
      onDismissStartedState.value()
    }

    fun cancelDismiss() {
      if (!dismissing || dismissed) {
        return
      }

      dismissing = false
      onDismissCancelledState.value()
    }

    fun requestDismiss() {
      if (dismissing || dismissed) {
        return
      }

      beginDismiss()
      coroutineScope.launch {
        try {
          anchoredState.animateTo(AnchorHidden, SheetAnimationSpec)
          finishDismiss()
        } catch (e: CancellationException) {
          if (!isActive) throw e
          if (anchoredState.targetValue != AnchorHidden) {
            cancelDismiss()
          }
        }
      }
    }

    LaunchedEffect(dismissing, sheetHasFocus) {
      if (dismissing && sheetHasFocus) {
        focusManager.clearFocus()
      }
    }

    LaunchedEffect(anchoredState) {
      snapshotFlow { anchoredState.targetValue }
        .collect { targetValue ->
          if (!hasSettledVisible) return@collect

          if (targetValue == AnchorHidden) {
            beginDismiss()
          } else {
            cancelDismiss()
          }
        }
    }

    LaunchedEffect(anchoredState) {
      var previousVisibleStop: Int? = null

      snapshotFlow { anchoredState.settledValue }
        .collect { settledValue ->
          val nextVisibleStop = settledValue.takeIf { it != AnchorHidden }
          if (nextVisibleStop != null) {
            hasSettledVisible = true
          } else if (hasSettledVisible || dismissing) {
            beginDismiss()
            finishDismiss()
          }
          if (
            previousVisibleStop != null &&
              nextVisibleStop != null &&
              nextVisibleStop != previousVisibleStop
          ) {
            hapticFeedbackState.value.performHapticFeedback(HapticFeedbackType.SegmentTick)
          }
          if (nextVisibleStop != null) {
            previousVisibleStop = nextVisibleStop
          }
          onSettledStopChangedState.value(nextVisibleStop)
        }
    }

    val scope = remember {
      object : AnchoredSheetSurfaceScope {
        override fun dismiss() {
          requestDismiss()
        }
      }
    }

    val nestedScrollConnection =
      rememberSheetNestedScrollConnection(
        anchoredState = anchoredState,
        visibleAnchors = visibleAnchors,
        containerHeightPx = containerHeightPx,
        hiddenValue = AnchorHidden,
        animationSpec = SheetAnimationSpec,
      )

    val stateOffset = if (anchoredState.offset.isNaN()) containerHeightPx else anchoredState.offset
    val offset = if (isIntrinsic) stateOffset else stateOffset + offsetCorrection.value
    val visibleHeight =
      resolveAnchoredSheetVisibleHeight(
        containerHeightPx = containerHeightPx,
        sheetOffsetPx = offset,
        density = density.density,
      )
    val animatedOffsetPx = offset.roundToInt().coerceAtLeast(0)
    val intrinsicTopLimit = intrinsicTopLimitPx.roundToInt()
    val minStopHeightPx =
      (containerHeightPx -
          (baseVisibleAnchors.maxOfOrNull(SheetAnchor::offset) ?: containerHeightPx))
        .roundToInt()
        .coerceAtLeast(0)
    val minVisibleOffset = topVisibleOffset
    val scrimAlpha =
      if (containerHeightPx > minVisibleOffset) {
        (1f - (offset - minVisibleOffset) / (containerHeightPx - minVisibleOffset)).coerceIn(0f, 1f)
      } else {
        0f
      }

    LaunchedEffect(visibleHeight) {
      onGeometryChangedState.value(AnchoredSheetGeometry(visibleHeight = visibleHeight))
    }

    scope.scrim(scrimAlpha)

    val sheetModifier =
      Modifier.fillMaxWidth()
        .nestedScroll(nestedScrollConnection)
        .layout { measurable, constraints ->
          val maxH =
            if (isIntrinsic) {
              (constraints.maxHeight - intrinsicTopLimit).coerceAtLeast(0)
            } else {
              maxOf((constraints.maxHeight - animatedOffsetPx).coerceAtLeast(0), minStopHeightPx)
            }
          val placeable = measurable.measure(constraints.copy(maxHeight = maxH))
          layout(placeable.width, placeable.height) { placeable.place(0, 0) }
        }
        .offset { IntOffset(x = 0, y = animatedOffsetPx) }
        .thenIf(isIntrinsic) {
          onSizeChanged {
            val measuredHeightPx = it.height.toFloat()
            if (contentHeightPx != measuredHeightPx) {
              contentHeightPx = measuredHeightPx
            }
          }
        }
        .safeDrawingHorizontalPadding()
        .clip(RoundedCornerShape(topStart = AppShapes.xl, topEnd = AppShapes.xl))

    Column(
      modifier =
        sheetModifier
          .anchoredDraggable(
            state = anchoredState,
            orientation = Orientation.Vertical,
            overscrollEffect = dragOverscrollEffect,
          )
          .onFocusChanged { sheetHasFocus = it.hasFocus }
    ) {
      scope.content()
    }
  }
}
