package co.typie.screen.editor.editor.subpane

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ui.component.sheet.SheetAnimationSpec
import co.typie.ui.theme.AppShapes
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

internal data class EditorResizableSheetGeometry(
  val sheetHeight: Float,
  val expandedHeight: Float,
  val visibleHeight: Float,
)

internal fun resolveEditorResizableSheetGeometry(
  sheetHeightPx: Float,
  expandedHeightPx: Float,
  keyboardOcclusionPx: Float,
  visibility: Float,
  density: Float,
): EditorResizableSheetGeometry {
  val safeDensity = density.takeIf { it > 0f } ?: 1f
  val visibleHeightPx = max(sheetHeightPx, keyboardOcclusionPx) * visibility.coerceIn(0f, 1f)
  return EditorResizableSheetGeometry(
    sheetHeight = sheetHeightPx / safeDensity,
    expandedHeight = expandedHeightPx / safeDensity,
    visibleHeight = visibleHeightPx / safeDensity,
  )
}

internal fun resolveKeyboardAwareSheetMinHeight(
  minHeightPx: Float,
  keyboardOcclusionPx: Float,
  minKeyboardVisibleHeightPx: Float,
  expandedHeightPx: Float,
): Float =
  max(minHeightPx, keyboardOcclusionPx + minKeyboardVisibleHeightPx).coerceAtMost(expandedHeightPx)

internal interface EditorResizableSheetSurfaceScope {
  fun dismiss()

  fun Modifier.sheetDragHandle(): Modifier
}

@Composable
internal fun EditorResizableSheetSurface(
  initialHeight: Dp,
  minHeight: Dp,
  dismissThreshold: Dp,
  maxTopInset: Dp,
  keyboardOcclusion: Dp,
  minKeyboardVisibleHeight: Dp,
  canDismiss: suspend () -> Boolean = { true },
  onDismissStarted: () -> Unit = {},
  onDismissed: () -> Unit,
  onGeometryChanged: (EditorResizableSheetGeometry) -> Unit,
  modifier: Modifier = Modifier,
  content: @Composable EditorResizableSheetSurfaceScope.() -> Unit,
) {
  BoxWithConstraints(modifier.fillMaxSize()) {
    val density = LocalDensity.current
    val coroutineScope = rememberCoroutineScope()
    val scrollGestureLockState = LocalScrollGestureLockState.current
    val canDismissState = rememberUpdatedState(canDismiss)
    val onDismissStartedState = rememberUpdatedState(onDismissStarted)
    val onDismissedState = rememberUpdatedState(onDismissed)
    val onGeometryChangedState = rememberUpdatedState(onGeometryChanged)
    val presentationProgress = remember { Animatable(1f) }
    val heightAnimation = remember { Animatable(0f) }
    var dismissRequestInProgress by remember { mutableStateOf(false) }
    var dismissing by remember { mutableStateOf(false) }
    var sheetDragScrollLock by remember { mutableStateOf<ScrollGestureLockHandle?>(null) }

    fun releaseSheetDragScrollLock() {
      sheetDragScrollLock?.release()
      sheetDragScrollLock = null
    }

    DisposableEffect(Unit) { onDispose { releaseSheetDragScrollLock() } }

    val containerHeightPx = with(density) { maxHeight.toPx() }
    val initialHeightPx = with(density) { initialHeight.toPx() }
    val minHeightPx = with(density) { minHeight.toPx() }
    val dismissThresholdPx = with(density) { dismissThreshold.toPx() }
    val maxTopInsetPx = with(density) { maxTopInset.toPx() }
    val keyboardOcclusionPx = with(density) { keyboardOcclusion.toPx() }
    val minKeyboardVisibleHeightPx = with(density) { minKeyboardVisibleHeight.toPx() }
    val dismissVelocityThresholdPx = with(density) { 1200.dp.toPx() }
    val expandedHeightPx = (containerHeightPx - maxTopInsetPx).coerceAtLeast(minHeightPx)
    val effectiveMinHeightPx =
      resolveKeyboardAwareSheetMinHeight(
        minHeightPx = minHeightPx,
        keyboardOcclusionPx = keyboardOcclusionPx,
        minKeyboardVisibleHeightPx = minKeyboardVisibleHeightPx,
        expandedHeightPx = expandedHeightPx,
      )

    var sheetHeightPx by remember { mutableFloatStateOf(Float.NaN) }

    fun resolvedSheetHeightPx(): Float =
      if (sheetHeightPx.isNaN()) {
        initialHeightPx.coerceIn(effectiveMinHeightPx, expandedHeightPx)
      } else {
        sheetHeightPx.coerceIn(0f, expandedHeightPx)
      }

    fun updateSheetHeight(value: Float) {
      sheetHeightPx = value.coerceIn(0f, expandedHeightPx)
    }

    suspend fun animateSheetHeightTo(target: Float) {
      heightAnimation.stop()
      heightAnimation.snapTo(resolvedSheetHeightPx())
      heightAnimation.animateTo(target.coerceIn(0f, expandedHeightPx), SheetAnimationSpec) {
        sheetHeightPx = value.coerceIn(0f, expandedHeightPx)
      }
    }

    LaunchedEffect(initialHeightPx, effectiveMinHeightPx, expandedHeightPx) {
      if (sheetHeightPx.isNaN()) {
        sheetHeightPx = initialHeightPx.coerceIn(effectiveMinHeightPx, expandedHeightPx)
        return@LaunchedEffect
      }

      when {
        sheetHeightPx > expandedHeightPx -> animateSheetHeightTo(expandedHeightPx)
        sheetHeightPx < effectiveMinHeightPx -> animateSheetHeightTo(effectiveMinHeightPx)
      }
    }

    val sheetHeight = resolvedSheetHeightPx()

    fun animateSheetHeight(target: Float) {
      coroutineScope.launch { animateSheetHeightTo(target) }
    }

    LaunchedEffect(Unit) { presentationProgress.animateTo(0f, SheetAnimationSpec) }

    LaunchedEffect(sheetHeight, expandedHeightPx, keyboardOcclusionPx, density.density) {
      snapshotFlow { presentationProgress.value }
        .collect { progress ->
          onGeometryChangedState.value(
            resolveEditorResizableSheetGeometry(
              sheetHeightPx = sheetHeight,
              expandedHeightPx = expandedHeightPx,
              keyboardOcclusionPx = keyboardOcclusionPx,
              visibility = 1f - progress,
              density = density.density,
            )
          )
        }
    }

    fun requestDismiss() {
      if (dismissing || dismissRequestInProgress) {
        return
      }

      dismissRequestInProgress = true
      coroutineScope.launch {
        try {
          if (!canDismissState.value()) {
            if (resolvedSheetHeightPx() < effectiveMinHeightPx) {
              animateSheetHeightTo(effectiveMinHeightPx)
            }
            return@launch
          }

          dismissing = true
          onDismissStartedState.value()
          heightAnimation.stop()
          presentationProgress.animateTo(1f, SheetAnimationSpec)
          onDismissedState.value()
        } finally {
          dismissRequestInProgress = false
        }
      }
    }

    val dragState = rememberDraggableState { delta ->
      if (!dismissing) {
        updateSheetHeight(resolvedSheetHeightPx() - delta)
      }
    }
    val scope =
      object : EditorResizableSheetSurfaceScope {
        override fun dismiss() {
          requestDismiss()
        }

        override fun Modifier.sheetDragHandle(): Modifier =
          draggable(
            state = dragState,
            orientation = Orientation.Vertical,
            enabled = !dismissing && !dismissRequestInProgress,
            onDragStarted = {
              heightAnimation.stop()
              releaseSheetDragScrollLock()
              sheetDragScrollLock = scrollGestureLockState.acquire()
            },
            onDragStopped = { velocity ->
              try {
                val shouldDismiss =
                  resolvedSheetHeightPx() <= dismissThresholdPx ||
                    velocity > dismissVelocityThresholdPx
                if (shouldDismiss) {
                  requestDismiss()
                } else if (resolvedSheetHeightPx() < effectiveMinHeightPx) {
                  animateSheetHeight(effectiveMinHeightPx)
                }
              } finally {
                releaseSheetDragScrollLock()
              }
            },
          )
      }

    val hiddenOffsetPx = sheetHeight * presentationProgress.value

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .height(with(density) { sheetHeight.toDp() })
          .align(Alignment.BottomCenter)
          .offset { IntOffset(x = 0, y = hiddenOffsetPx.roundToInt()) }
          .clip(RoundedCornerShape(topStart = AppShapes.xl, topEnd = AppShapes.xl))
          .blockPointerInputBehind()
    ) {
      scope.content()
    }
  }
}

private fun Modifier.blockPointerInputBehind(): Modifier = pointerInput(Unit) {}
