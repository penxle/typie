package co.typie.screen.editor.editor.subpane

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.gestures.waitForUpOrCancellation
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
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ui.component.sheet.SheetAnimationSpec
import co.typie.ui.component.sheet.SheetBarDefaults
import co.typie.ui.component.sheet.SheetHandleContainerHeight
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

internal data class EditorResizableSheetGeometry(
  val sheetHeight: Float,
  val expandedHeight: Float,
  val visibleHeight: Float,
)

private const val EditorSubPaneHazeZIndex = 1f
internal val EditorSubPaneBarHeight = SheetBarDefaults.SlotWidth
private val EditorSubPaneHeaderBottomClearance = 4.dp
internal val EditorSubPaneHeaderRevealHeight =
  SheetHandleContainerHeight + EditorSubPaneBarHeight + EditorSubPaneHeaderBottomClearance

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
  val keyboardOcclusion: Dp

  fun dismiss()

  fun Modifier.sheetDragHandle(): Modifier
}

@Composable
internal fun EditorResizableSheetSurface(
  initialHeight: Dp,
  minHeight: Dp,
  dismissThreshold: Dp,
  maxTopInset: Dp,
  trustedImeBottomInset: Dp,
  safeBottomInset: Dp,
  editorFocused: Boolean,
  foregroundOcclusion: EditorSubPaneForegroundOcclusion =
    EditorSubPaneForegroundOcclusion(height = trustedImeBottomInset),
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
    val focusManager = LocalFocusManager.current
    val sheetFocusRequester = remember { FocusRequester() }
    val toolbarBackdropHazeState = LocalHazeState.current
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
    var sheetHasFocus by remember { mutableStateOf(false) }
    var sheetSurfaceFocused by remember { mutableStateOf(false) }
    var sheetDragInProgress by remember { mutableStateOf(false) }
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
    val rawKeyboardOcclusion = (trustedImeBottomInset - safeBottomInset).coerceAtLeast(0.dp)
    val sheetDescendantHasFocus = sheetHasFocus && !sheetSurfaceFocused
    val effectiveKeyboardOcclusion = if (sheetDescendantHasFocus) rawKeyboardOcclusion else 0.dp
    val effectiveKeyboardOcclusionPx = with(density) { effectiveKeyboardOcclusion.toPx() }
    val foregroundOcclusionHeightPx =
      with(density) { foregroundOcclusion.height.toPx() }.coerceAtLeast(0f)
    val foregroundHeaderRevealHeightPx =
      with(density) { foregroundOcclusion.headerRevealHeight.toPx() }.coerceAtLeast(0f)
    val rawForegroundOcclusion = (foregroundOcclusion.height - safeBottomInset).coerceAtLeast(0.dp)
    val renderedHeightOverrideActive =
      editorFocused && !sheetHasFocus && rawForegroundOcclusion > 0.dp
    val minKeyboardVisibleHeightPx = with(density) { minKeyboardVisibleHeight.toPx() }
    val dismissVelocityThresholdPx = with(density) { 1200.dp.toPx() }
    val expandedHeightPx = (containerHeightPx - maxTopInsetPx).coerceAtLeast(minHeightPx)
    val effectiveMinHeightPx =
      resolveKeyboardAwareSheetMinHeight(
        minHeightPx = minHeightPx,
        keyboardOcclusionPx = effectiveKeyboardOcclusionPx,
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

    val preferredSheetHeight = resolvedSheetHeightPx()
    val renderedHeightTarget =
      if (renderedHeightOverrideActive) {
        if (foregroundHeaderRevealHeightPx > 0f) {
          (foregroundOcclusionHeightPx + foregroundHeaderRevealHeightPx).coerceAtMost(
            expandedHeightPx
          )
        } else {
          minOf(preferredSheetHeight, foregroundOcclusionHeightPx)
        }
      } else {
        preferredSheetHeight
      }
    val renderedHeightAnimation = remember { Animatable(preferredSheetHeight) }
    val retainedRenderedHeightVelocity = remember { FloatArray(1) }
    var renderedHeightTransitionFromOverride by remember {
      mutableStateOf(renderedHeightOverrideActive)
    }

    LaunchedEffect(
      renderedHeightTarget,
      renderedHeightOverrideActive,
      preferredSheetHeight,
      sheetDragInProgress,
    ) {
      when {
        sheetDragInProgress -> {
          renderedHeightTransitionFromOverride = false
          retainedRenderedHeightVelocity[0] = 0f
          renderedHeightAnimation.snapTo(preferredSheetHeight)
        }
        renderedHeightOverrideActive -> {
          renderedHeightTransitionFromOverride = true
          renderedHeightAnimation.animateTo(
            targetValue = renderedHeightTarget,
            animationSpec = SheetAnimationSpec,
            initialVelocity = retainedRenderedHeightVelocity[0],
          ) {
            retainedRenderedHeightVelocity[0] = velocity
          }
          retainedRenderedHeightVelocity[0] = 0f
        }
        renderedHeightTransitionFromOverride -> {
          renderedHeightAnimation.animateTo(
            targetValue = preferredSheetHeight,
            animationSpec = SheetAnimationSpec,
            initialVelocity = retainedRenderedHeightVelocity[0],
          ) {
            retainedRenderedHeightVelocity[0] = velocity
          }
          retainedRenderedHeightVelocity[0] = 0f
          renderedHeightTransitionFromOverride = false
        }
        else -> {
          retainedRenderedHeightVelocity[0] = 0f
          renderedHeightAnimation.snapTo(preferredSheetHeight)
        }
      }
    }

    val renderedSheetHeight =
      (if (sheetDragInProgress) preferredSheetHeight else renderedHeightAnimation.value).coerceIn(
        0f,
        expandedHeightPx,
      )

    fun animateSheetHeight(target: Float) {
      coroutineScope.launch { animateSheetHeightTo(target) }
    }

    LaunchedEffect(Unit) { presentationProgress.animateTo(0f, SheetAnimationSpec) }

    LaunchedEffect(
      renderedSheetHeight,
      expandedHeightPx,
      effectiveKeyboardOcclusionPx,
      density.density,
    ) {
      snapshotFlow { presentationProgress.value }
        .collect { progress ->
          onGeometryChangedState.value(
            resolveEditorResizableSheetGeometry(
              sheetHeightPx = renderedSheetHeight,
              expandedHeightPx = expandedHeightPx,
              keyboardOcclusionPx = effectiveKeyboardOcclusionPx,
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
          if (sheetHasFocus) {
            focusManager.clearFocus()
          }
          onDismissStartedState.value()
          heightAnimation.stop()
          presentationProgress.animateTo(1f, SheetAnimationSpec)
          onDismissedState.value()
        } finally {
          dismissRequestInProgress = false
        }
      }
    }

    LaunchedEffect(dismissing, sheetHasFocus) {
      if (dismissing && sheetHasFocus) {
        focusManager.clearFocus()
      }
    }

    val dragState = rememberDraggableState { delta ->
      if (!dismissing) {
        updateSheetHeight(resolvedSheetHeightPx() - delta)
      }
    }
    val scope =
      object : EditorResizableSheetSurfaceScope {
        override val keyboardOcclusion: Dp
          get() = effectiveKeyboardOcclusion

        override fun dismiss() {
          requestDismiss()
        }

        override fun Modifier.sheetDragHandle(): Modifier =
          draggable(
              state = dragState,
              orientation = Orientation.Vertical,
              enabled = !dismissing && !dismissRequestInProgress,
              onDragStarted = {
                val draggingFromTemporaryReveal =
                  renderedHeightOverrideActive && foregroundHeaderRevealHeightPx > 0f
                val dragStartHeight =
                  if (draggingFromTemporaryReveal) {
                    renderedHeightAnimation.value.coerceIn(0f, expandedHeightPx)
                  } else {
                    resolvedSheetHeightPx()
                  }
                if (draggingFromTemporaryReveal) {
                  updateSheetHeight(dragStartHeight)
                }
                sheetFocusRequester.requestFocus()
                sheetDragInProgress = true
                renderedHeightTransitionFromOverride = false
                retainedRenderedHeightVelocity[0] = 0f
                renderedHeightAnimation.stop()
                renderedHeightAnimation.snapTo(dragStartHeight)
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
                  sheetDragInProgress = false
                  releaseSheetDragScrollLock()
                }
              },
            )
            .pointerInput(dismissing, dismissRequestInProgress) {
              if (!dismissing && !dismissRequestInProgress) {
                awaitEachGesture {
                  awaitFirstDown(requireUnconsumed = false)
                  if (waitForUpOrCancellation() != null) {
                    sheetFocusRequester.requestFocus()
                  }
                }
              }
            }
      }

    val hiddenOffsetPx = renderedSheetHeight * presentationProgress.value

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .height(with(density) { renderedSheetHeight.toDp() })
          .align(Alignment.BottomCenter)
          .offset { IntOffset(x = 0, y = hiddenOffsetPx.roundToInt()) }
          .clip(RoundedCornerShape(topStart = AppShapes.xl, topEnd = AppShapes.xl))
          .hazeSource(toolbarBackdropHazeState, zIndex = EditorSubPaneHazeZIndex)
          .focusRequester(sheetFocusRequester)
          .onFocusChanged {
            sheetHasFocus = it.hasFocus
            sheetSurfaceFocused = it.isFocused
          }
          .focusable(enabled = !dismissing)
          .blockPointerInputBehind()
    ) {
      scope.content()
    }
  }
}

private fun Modifier.blockPointerInputBehind(): Modifier = pointerInput(Unit) {}
