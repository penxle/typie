package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.foundation.gestures.scrollable2D
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import co.typie.editor.ext.unclippedBoundsInRoot
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorScrollIntentResult
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.isEditorScrollTargetVisible
import co.typie.editor.scroll.resolveEditorScrollIntent
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.consumeEditorViewportWheelPan
import co.typie.navigation.navigationPopNestedScroll
import co.typie.screen.editor.editor.overlay.editorMagnifier
import co.typie.screen.editor.editor.overlay.resolveEditorMagnifierPlacement
import co.typie.screen.editor.editor.state.EditorScreenState
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource
import kotlin.coroutines.coroutineContext
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

private enum class EditorScreenLayoutSlot {
  ViewportContent,
  ViewportOverlay,
  Overlay,
  Toolbar,
  SubPane,
}

private const val EditorViewportScrollAnchorTolerance = 1f

internal enum class EditorViewportScrollReconcileMode {
  Disabled,
  KeepVisibleAnchor,
  RevealSelectionHead,
}

@Composable
internal fun EditorScreenLayout(
  state: EditorScreenState,
  scrollFrame: EditorScrollFrame,
  visibleArea: EditorVisibleArea,
  magnifierFocalPositionInRoot: Offset? = null,
  viewportScrollableState: Scrollable2DState,
  viewportContentWidth: Float,
  viewportInputEnabled: Boolean = true,
  viewportScrollReconcileMode: EditorViewportScrollReconcileMode,
  onViewportWheelScroll: () -> Unit = {},
  onMeasuredViewportSizeChange: (Size) -> Unit,
  header: @Composable () -> Unit,
  body: @Composable () -> Unit,
  viewportOverlay: @Composable BoxScope.() -> Unit = {},
  overlay: @Composable () -> Unit = {},
  toolbar: @Composable () -> Unit,
  subPane: @Composable BoxScope.() -> Unit = {},
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val toolbarBackdropHazeState = LocalHazeState.current
  val scrollReconcileState = remember { EditorViewportScrollReconcileState() }
  val coroutineScope = rememberCoroutineScope()
  var smoothScrollJob by remember { mutableStateOf<Job?>(null) }
  var layoutBoundsInRoot by remember { mutableStateOf<Rect?>(null) }
  LaunchedEffect(
    state.viewportState.isTransforming,
    state.viewportState.isDirectManipulationInProgress,
  ) {
    if (state.viewportState.isTransforming || state.viewportState.isDirectManipulationInProgress) {
      smoothScrollJob?.cancel()
      smoothScrollJob = null
    }
  }
  val magnifierPlacement = layoutBoundsInRoot?.let { bounds ->
    val focalPositionInRoot = magnifierFocalPositionInRoot ?: return@let null
    resolveEditorMagnifierPlacement(
      focalPosition =
        Offset(x = focalPositionInRoot.x - bounds.left, y = focalPositionInRoot.y - bounds.top),
      overlaySize = bounds.size,
      visibleArea = visibleArea,
      density = density.density,
    )
  }
  val resolveSize: (Int, Int) -> Size =
    remember(density) {
      { width, height -> Size(width = width / density.density, height = height / density.density) }
    }

  SubcomposeLayout(
    modifier =
      modifier.fillMaxSize().editorMagnifier(magnifierPlacement).onGloballyPositioned { coordinates
        ->
        layoutBoundsInRoot = coordinates.unclippedBoundsInRoot()
      }
  ) { constraints ->
    val viewportWidth = constraints.maxWidth / density.density
    val resolvedContentWidth =
      resolveEditorViewportContentWidth(
        viewportWidth = viewportWidth,
        contentTrackWidth = viewportContentWidth,
      )
    val toolbarPlaceables =
      subcompose(EditorScreenLayoutSlot.Toolbar, toolbar).map {
        it.measure(constraints.copy(minWidth = 0, minHeight = 0))
      }
    val viewportHeight = constraints.maxHeight
    val viewportConstraints =
      constraints.copy(
        minWidth = constraints.maxWidth,
        maxWidth = constraints.maxWidth,
        minHeight = viewportHeight,
        maxHeight = viewportHeight,
      )
    val viewportContentPlaceables =
      subcompose(EditorScreenLayoutSlot.ViewportContent) {
          val viewportInputModifier =
            if (viewportInputEnabled) {
              Modifier.scrollable2D(state = viewportScrollableState)
                .editorViewportWheelScroll(
                  viewportState = state.viewportState,
                  onScrollConsumed = onViewportWheelScroll,
                )
            } else {
              Modifier
            }
          Layout(
            modifier =
              Modifier.fillMaxSize()
                .clipToBounds()
                .hazeSource(toolbarBackdropHazeState)
                .navigationPopNestedScroll()
                .then(viewportInputModifier),
            content = {
              Column {
                Box(modifier = Modifier.width(viewportWidth.dp)) { header() }
                Box(
                  modifier =
                    Modifier.fillMaxWidth().graphicsLayer {
                      translationX = -state.viewportState.scrollOffset.x * density.density
                    }
                ) {
                  body()
                }
              }
            },
          ) { measurables, viewportConstraints ->
            val contentConstraints =
              resolveEditorViewportContentConstraints(
                viewportWidthPx = viewportConstraints.maxWidth,
                contentWidthPx = resolvedContentWidth.dp.roundToPx(),
              )
            val placeable = measurables.single().measure(contentConstraints)
            val measuredViewportSize =
              resolveSize(viewportConstraints.maxWidth, viewportConstraints.maxHeight)
            val viewportSizeChanged =
              state.viewportState.updateMeasuredBounds(
                viewportSize = measuredViewportSize,
                contentSize = resolveSize(placeable.width, placeable.height),
              )
            if (viewportSizeChanged) {
              onMeasuredViewportSizeChange(measuredViewportSize)
            }
            val scrollFrameVersion = scrollFrame.state.version
            val bringIntoViewRequest =
              bringIntoViewRequests.activateForVersion(version = scrollFrameVersion)
            if (bringIntoViewRequest != null) {
              val bringIntoViewTarget = bringIntoViewRequest.target
              if (
                state.viewportState.isTransforming ||
                  state.viewportState.isDirectManipulationInProgress
              ) {
                bringIntoViewRequests.cancel()
              } else {
                when (
                  val scrollIntentResult =
                    resolveEditorScrollIntent(
                      frame = scrollFrame,
                      target = bringIntoViewTarget,
                      currentScroll = state.viewportState.scrollOffset.y,
                    )
                ) {
                  EditorScrollIntentResult.Unresolved -> Unit
                  EditorScrollIntentResult.ConsumedWithoutScroll -> {
                    bringIntoViewRequests.markApplied(
                      version = scrollFrameVersion,
                      request = bringIntoViewRequest,
                    )
                  }
                  is EditorScrollIntentResult.ScrollTo -> {
                    if (
                      bringIntoViewRequests.markApplied(
                        version = scrollFrameVersion,
                        request = bringIntoViewRequest,
                      )
                    ) {
                      smoothScrollJob?.cancel()
                      smoothScrollJob =
                        when (bringIntoViewRequest.behavior) {
                          EditorBringIntoViewBehavior.Instant -> {
                            state.viewportState.scrollToY(
                              targetY = scrollIntentResult.y,
                              isAutoScroll = true,
                            )
                            null
                          }

                          EditorBringIntoViewBehavior.Smooth ->
                            coroutineScope.launch {
                              try {
                                state.viewportState.animateScrollToY(
                                  targetY = scrollIntentResult.y,
                                  isAutoScroll = true,
                                )
                              } finally {
                                if (smoothScrollJob == coroutineContext[Job]) {
                                  smoothScrollJob = null
                                }
                              }
                            }
                        }
                    }
                  }
                }
              }
            } else {
              scrollReconcileState.reconcile(
                mode = viewportScrollReconcileMode,
                viewportState = state.viewportState,
                scrollFrame = scrollFrame,
                visibleArea = visibleArea,
              )
            }

            layout(width = viewportConstraints.maxWidth, height = viewportConstraints.maxHeight) {
              val scrollY = (state.viewportState.scrollOffset.y * density.density).roundToInt()
              placeable.place(x = 0, y = -scrollY)
            }
          }
        }
        .map { it.measure(viewportConstraints) }
    val viewportOverlayPlaceables =
      subcompose(EditorScreenLayoutSlot.ViewportOverlay) {
          Box(modifier = Modifier.fillMaxSize().clipToBounds(), content = viewportOverlay)
        }
        .map { it.measure(viewportConstraints) }
    val overlayPlaceables =
      subcompose(EditorScreenLayoutSlot.Overlay, overlay).map {
        it.measure(
          constraints.copy(
            minWidth = constraints.maxWidth,
            maxWidth = constraints.maxWidth,
            minHeight = constraints.maxHeight,
            maxHeight = constraints.maxHeight,
          )
        )
      }
    val subPanePlaceables =
      subcompose(EditorScreenLayoutSlot.SubPane) {
          Box(modifier = Modifier.fillMaxSize(), content = subPane)
        }
        .map {
          it.measure(
            constraints.copy(
              minWidth = constraints.maxWidth,
              maxWidth = constraints.maxWidth,
              minHeight = constraints.maxHeight,
              maxHeight = constraints.maxHeight,
            )
          )
        }

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      viewportContentPlaceables.forEach { it.place(x = 0, y = 0) }
      viewportOverlayPlaceables.forEach { it.place(x = 0, y = 0) }
      overlayPlaceables.forEach { it.place(x = 0, y = 0) }
      toolbarPlaceables.forEach { it.place(x = 0, y = constraints.maxHeight - it.height) }
      subPanePlaceables.forEach { it.place(x = 0, y = 0) }
    }
  }
}

internal class EditorViewportScrollReconcileState {
  private var lastObservedFrame: EditorViewportScrollReconcileFrame? = null

  fun reset() {
    lastObservedFrame = null
  }

  fun reconcile(
    mode: EditorViewportScrollReconcileMode,
    viewportState: EditorViewportState,
    scrollFrame: EditorScrollFrame,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    if (mode == EditorViewportScrollReconcileMode.Disabled) {
      reset()
      return false
    }
    if (viewportState.isTransforming || viewportState.isDirectManipulationInProgress) {
      return false
    }

    val frame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
    val previousFrame = lastObservedFrame
    if (previousFrame == null) {
      lastObservedFrame = frame
      return false
    }
    if (previousFrame.hasSameVisibleViewport(frame)) {
      lastObservedFrame = frame
      return false
    }

    return when (mode) {
      EditorViewportScrollReconcileMode.Disabled -> false
      EditorViewportScrollReconcileMode.KeepVisibleAnchor ->
        reconcileKeepVisibleAnchor(
          previousFrame = previousFrame,
          frame = frame,
          viewportState = viewportState,
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
        )
      EditorViewportScrollReconcileMode.RevealSelectionHead ->
        reconcileSelectionHeadReveal(
          scrollFrame = scrollFrame,
          viewportState = viewportState,
          visibleArea = visibleArea,
        )
    }
  }

  private fun reconcileKeepVisibleAnchor(
    previousFrame: EditorViewportScrollReconcileFrame,
    frame: EditorViewportScrollReconcileFrame,
    viewportState: EditorViewportState,
    scrollFrame: EditorScrollFrame,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    // visible area가 바뀌기 전 selection head가 이미 보이던 상태라면, 화면 중앙을 보존하지
    // 않고 selection head가 새 keep-visible 범위 안에 남도록만 보정한다.
    val wasSelectionVisible =
      isEditorScrollTargetVisible(
        frame = scrollFrame,
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        currentScroll = previousFrame.scrollY,
        visibleArea = previousFrame.visibleArea,
      )
    if (wasSelectionVisible == true) {
      return reconcileSelectionHeadReveal(
        scrollFrame = scrollFrame,
        viewportState = viewportState,
        visibleArea = visibleArea,
      )
    }

    val anchor = previousFrame.anchor
    val targetY = previousFrame.documentY(anchor) - frame.viewportY(anchor)
    if (abs(targetY - viewportState.scrollOffset.y) <= EditorViewportScrollAnchorTolerance) {
      lastObservedFrame = frame
      return false
    }

    viewportState.scrollToY(targetY = targetY, isAutoScroll = true)
    lastObservedFrame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
    return true
  }

  private fun reconcileSelectionHeadReveal(
    scrollFrame: EditorScrollFrame,
    viewportState: EditorViewportState,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    return when (
      val scrollIntentResult =
        resolveEditorScrollIntent(
          frame = scrollFrame,
          target = EditorBringIntoViewTarget.CurrentSelectionHead,
          currentScroll = viewportState.scrollOffset.y,
        )
    ) {
      EditorScrollIntentResult.Unresolved,
      EditorScrollIntentResult.ConsumedWithoutScroll -> {
        lastObservedFrame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
        false
      }
      is EditorScrollIntentResult.ScrollTo -> {
        viewportState.scrollToY(targetY = scrollIntentResult.y, isAutoScroll = true)
        lastObservedFrame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
        true
      }
    }
  }
}

private enum class EditorViewportScrollAnchor {
  Top,
  Center,
  Bottom,
}

private data class EditorViewportScrollReconcileFrame(
  val scrollY: Float,
  val maxScrollY: Float,
  val visibleArea: EditorVisibleArea,
  val visibleViewportTop: Float,
  val visibleViewportBottom: Float,
) {
  constructor(
    viewportState: EditorViewportState,
    visibleArea: EditorVisibleArea,
  ) : this(
    scrollY = viewportState.scrollOffset.y,
    maxScrollY = viewportState.maxScrollY,
    visibleArea = visibleArea,
    visibleViewportTop = visibleArea.visibleViewportTop,
    visibleViewportBottom = visibleArea.visibleViewportBottom,
  )

  val anchor: EditorViewportScrollAnchor
    get() =
      when {
        scrollY <= EditorViewportScrollAnchorTolerance -> EditorViewportScrollAnchor.Top
        maxScrollY - scrollY <= EditorViewportScrollAnchorTolerance ->
          EditorViewportScrollAnchor.Bottom
        else -> EditorViewportScrollAnchor.Center
      }

  fun hasSameVisibleViewport(other: EditorViewportScrollReconcileFrame): Boolean =
    visibleViewportTop == other.visibleViewportTop &&
      visibleViewportBottom == other.visibleViewportBottom

  fun documentY(anchor: EditorViewportScrollAnchor): Float = scrollY + viewportY(anchor)

  fun viewportY(anchor: EditorViewportScrollAnchor): Float =
    when (anchor) {
      EditorViewportScrollAnchor.Top -> visibleViewportTop
      EditorViewportScrollAnchor.Center -> (visibleViewportTop + visibleViewportBottom) / 2f
      EditorViewportScrollAnchor.Bottom -> visibleViewportBottom
    }
}

internal fun resolveEditorViewportContentWidth(
  viewportWidth: Float,
  contentTrackWidth: Float,
): Float = max(viewportWidth, contentTrackWidth).coerceAtLeast(0f)

internal fun resolveEditorViewportContentConstraints(
  viewportWidthPx: Int,
  contentWidthPx: Int,
): Constraints {
  val resolvedWidth = max(viewportWidthPx, contentWidthPx).coerceAtLeast(0)
  return Constraints(
    minWidth = resolvedWidth,
    maxWidth = resolvedWidth,
    minHeight = 0,
    maxHeight = Constraints.Infinity,
  )
}

private fun Modifier.editorViewportWheelScroll(
  viewportState: EditorViewportState,
  onScrollConsumed: () -> Unit,
): Modifier =
  pointerInput(viewportState, onScrollConsumed) {
    awaitPointerEventScope {
      while (true) {
        val event = awaitPointerEvent(PointerEventPass.Main)
        if (event.type != PointerEventType.Scroll) {
          continue
        }
        if (event.keyboardModifiers.isCtrlPressed || event.keyboardModifiers.isMetaPressed) {
          continue
        }

        val scrollDelta =
          event.changes.fold(Offset.Zero) { delta, change ->
            if (change.isConsumed) {
              delta
            } else {
              delta + change.scrollDelta
            }
          }
        if (scrollDelta == Offset.Zero) {
          continue
        }

        // DesktopScrollTranslation turns mouse drags into synthetic wheel events; handle those
        // here
        // as the same viewport pan path because scrollable2D currently has no wheel handling.
        viewportState.updateScrollableInteractionInProgress(true)
        val consumed =
          consumeEditorViewportWheelPan(viewportState = viewportState, scrollDelta = scrollDelta)
        viewportState.updateScrollableInteractionInProgress(false)
        if (consumed != Offset.Zero) {
          onScrollConsumed()
          event.changes.forEach { it.consume() }
        }
      }
    }
  }
