package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import co.typie.editor.Editor
import co.typie.editor.interaction.EditorInteractionController
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.EditorPointerCoordinateResolver
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.consumeEditorViewportWheelPan
import co.typie.screen.editor.editor.viewport.editorPointerSignalWheelZoom
import kotlin.math.roundToInt

@Composable
internal fun EditorOverlayPointerRouter(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
  viewportState: EditorViewportState,
) {
  if (!uiState.focused || density <= 0f) {
    return
  }

  val interactionScope = LocalEditorInteractionScope.current
  val interactionController = interactionScope.controller
  var activeTarget by remember { mutableStateOf<EditorOverlayPointerTarget?>(null) }
  val targets =
    retainActiveOverlayPointerTarget(
      currentTargets =
        resolveOverlayPointerTargets(
          editor = editor,
          uiState = uiState,
          editorRectInOverlay = editorRectInOverlay,
          density = density,
        ),
      activeTarget = activeTarget,
    )

  targets.forEach { target ->
    key(target.key) {
      EditorOverlayPointerRouterTarget(
        target = target,
        editorRectInOverlay = editorRectInOverlay,
        density = density,
        viewportState = viewportState,
        interactionScope = interactionScope,
        interactionController = interactionController,
        onPointerStreamStart = { activeTarget = it },
        onPointerStreamEnd = { activeTarget = null },
      )
    }
  }
}

private data class EditorOverlayPointerTarget(val key: String, val rectInOverlay: Rect)

private fun retainActiveOverlayPointerTarget(
  currentTargets: List<EditorOverlayPointerTarget>,
  activeTarget: EditorOverlayPointerTarget?,
): List<EditorOverlayPointerTarget> {
  activeTarget ?: return currentTargets

  var retained = false
  val targets = currentTargets.map { target ->
    if (target.key == activeTarget.key) {
      retained = true
      activeTarget
    } else {
      target
    }
  }
  return if (retained) targets else currentTargets + activeTarget
}

private fun resolveOverlayPointerTargets(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
): List<EditorOverlayPointerTarget> =
  buildList {
      resolveTableCellSelectionOverlayPlacements(
          editor = editor,
          uiState = uiState,
          editorRectInOverlay = editorRectInOverlay,
          density = density,
        )
        .forEach { placement ->
          placement.handleTouchTargetRect(density)?.let { rect ->
            add(
              EditorOverlayPointerTarget(
                key = "table-cell:${placement.tableId}",
                rectInOverlay = rect,
              )
            )
          }
        }

      resolveSelectionHandleOverlayPlacements(
          editor = editor,
          uiState = uiState,
          editorRectInOverlay = editorRectInOverlay,
          density = density,
        )
        ?.forEach { placement ->
          val geometry =
            resolveSelectionHandleOverlayGeometry(placement = placement, density = density)
          add(
            EditorOverlayPointerTarget(
              key = "selection:${placement.type}",
              rectInOverlay =
                Rect(
                  topLeft = geometry.touchTargetTopLeft,
                  bottomRight =
                    geometry.touchTargetTopLeft +
                      Offset(
                        x = geometry.touchTargetSize.width,
                        y = geometry.touchTargetSize.height,
                      ),
                ),
            )
          )
        }
    }
    .filter { target -> target.rectInOverlay.width > 0f && target.rectInOverlay.height > 0f }

@Composable
private fun EditorOverlayPointerRouterTarget(
  target: EditorOverlayPointerTarget,
  editorRectInOverlay: Rect,
  density: Float,
  viewportState: EditorViewportState,
  interactionScope: EditorInteractionScope,
  interactionController: EditorInteractionController,
  onPointerStreamStart: (EditorOverlayPointerTarget) -> Unit,
  onPointerStreamEnd: () -> Unit,
) {
  val localDensity = LocalDensity.current
  val currentTarget by rememberUpdatedState(target)

  Box(
    modifier =
      Modifier.offset {
          IntOffset(target.rectInOverlay.left.roundToInt(), target.rectInOverlay.top.roundToInt())
        }
        .size(
          width = with(localDensity) { target.rectInOverlay.width.toDp() },
          height = with(localDensity) { target.rectInOverlay.height.toDp() },
        )
        .pointerInput(target.key) {
          awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false)
            onPointerStreamStart(currentTarget)
            try {
              do {
                val event = awaitPointerEvent()
              } while (event.changes.any { change -> change.pressed })
            } finally {
              onPointerStreamEnd()
            }
          }
        }
        .editorOverlayViewportWheelInput(
          viewportState = viewportState,
          interactionScope = interactionScope,
          targetRectInOverlay = target.rectInOverlay,
        )
        .editorInteractions(
          density = density,
          interactionController = interactionController,
          coordinateResolver =
            EditorOverlayPointerTargetCoordinateResolver(
              editorRectInOverlay = editorRectInOverlay,
              targetRectInOverlay = target.rectInOverlay,
            ),
        )
  )
}

internal fun Modifier.editorOverlayViewportWheelInput(
  viewportState: EditorViewportState,
  interactionScope: EditorInteractionScope,
  targetRectInOverlay: Rect,
): Modifier =
  editorPointerSignalWheelZoom(
      key1 = interactionScope,
      key2 = targetRectInOverlay,
      onZoomSessionStart = interactionScope::beginPointerSignalZoom,
      onZoom = { focalPosition, normalizedDelta ->
        interactionScope.updatePointerSignalZoom(
          focalPosition = targetRectInOverlay.topLeft + focalPosition,
          normalizedDelta = normalizedDelta,
        )
      },
      onZoomSessionEnd = interactionScope::endPointerSignalZoom,
    )
    .pointerInput(viewportState) {
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

internal class EditorOverlayPointerTargetCoordinateResolver(
  private val editorRectInOverlay: Rect,
  private val targetRectInOverlay: Rect,
) : EditorPointerCoordinateResolver {
  override fun positionForPointerStart(position: Offset): Offset = positionInEditor(position)

  override fun positionForTapStart(position: Offset): Offset = positionInEditor(position)

  override fun positionForActivePointer(position: Offset): Offset = positionInEditor(position)

  private fun positionInEditor(position: Offset): Offset =
    targetRectInOverlay.topLeft + position - editorRectInOverlay.topLeft
}
