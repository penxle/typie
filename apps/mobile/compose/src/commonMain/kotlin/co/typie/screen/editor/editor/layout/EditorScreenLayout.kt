package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.foundation.gestures.scrollable2D
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.consumeEditorViewportWheelPan
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.max

private enum class EditorScreenLayoutSlot {
  ViewportContent,
  ViewportOverlay,
  Overlay,
  Toolbar,
}

@Composable
internal fun EditorScreenLayout(
  state: EditorScreenState,
  viewportScrollableState: Scrollable2DState,
  viewportContentWidth: Float,
  onViewportSizeChange: (Size) -> Unit,
  header: @Composable () -> Unit,
  body: @Composable () -> Unit,
  viewportOverlay: @Composable BoxScope.() -> Unit = {},
  overlay: @Composable () -> Unit = {},
  toolbar: @Composable () -> Unit,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val resolveSize: (Int, Int) -> Size =
    remember(density) {
      { width, height -> Size(width = width / density.density, height = height / density.density) }
    }

  SubcomposeLayout(modifier = modifier.fillMaxSize()) { constraints ->
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
    val toolbarHeight = toolbarPlaceables.maxOfOrNull { it.height } ?: 0
    val viewportHeight = (constraints.maxHeight - toolbarHeight).coerceAtLeast(0)
    val viewportConstraints =
      constraints.copy(
        minWidth = constraints.maxWidth,
        maxWidth = constraints.maxWidth,
        minHeight = viewportHeight,
        maxHeight = viewportHeight,
      )
    val viewportContentPlaceables =
      subcompose(EditorScreenLayoutSlot.ViewportContent) {
          Layout(
            modifier =
              Modifier.fillMaxSize()
                .clipToBounds()
                .scrollable2D(state = viewportScrollableState)
                .editorViewportWheelScroll(state.viewportState)
                .onSizeChanged { size ->
                  onViewportSizeChange(resolveSize(size.width, size.height))
                },
            content = {
              Column(
                modifier =
                  Modifier.graphicsLayer {
                      translationX = -state.viewportState.scrollOffset.x * density.density
                      translationY = -state.viewportState.scrollOffset.y * density.density
                    }
                    .onSizeChanged { size ->
                      state.viewportState.updateContentSize(resolveSize(size.width, size.height))
                    }
              ) {
                header()
                body()
              }
            },
          ) { measurables, viewportConstraints ->
            val contentConstraints =
              resolveEditorViewportContentConstraints(
                viewportWidthPx = viewportConstraints.maxWidth,
                contentWidthPx = resolvedContentWidth.dp.roundToPx(),
              )
            val placeable = measurables.single().measure(contentConstraints)

            layout(width = viewportConstraints.maxWidth, height = viewportConstraints.maxHeight) {
              placeable.place(x = 0, y = 0)
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

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      viewportContentPlaceables.forEach { it.place(x = 0, y = 0) }
      viewportOverlayPlaceables.forEach { it.place(x = 0, y = 0) }
      overlayPlaceables.forEach { it.place(x = 0, y = 0) }
      toolbarPlaceables.forEach { it.place(x = 0, y = constraints.maxHeight - it.height) }
    }
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

private fun Modifier.editorViewportWheelScroll(viewportState: EditorViewportState): Modifier =
  pointerInput(viewportState) {
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

        // DesktopScrollTranslation turns mouse drags into synthetic wheel events; handle those here
        // as the same viewport pan path because scrollable2D currently has no wheel handling.
        viewportState.updateScrollableInteractionInProgress(true)
        val consumed =
          consumeEditorViewportWheelPan(viewportState = viewportState, scrollDelta = scrollDelta)
        viewportState.updateScrollableInteractionInProgress(false)
        if (consumed != Offset.Zero) {
          event.changes.forEach { it.consume() }
        }
      }
    }
  }
