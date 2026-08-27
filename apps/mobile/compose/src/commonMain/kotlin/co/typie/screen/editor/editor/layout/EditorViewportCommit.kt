package co.typie.screen.editor.editor.layout

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Size
import co.typie.editor.Editor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveContinuousLayoutViewportWidth
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent

@Composable
internal fun rememberCommittedEditorRenderZoom(
  editor: Editor?,
  physicalViewport: Size,
  layoutSpec: EditorDocumentLayoutSpec,
  requestedRenderZoom: Float,
  scaleFactor: Double,
): Float {
  // Continuous surface scale must not advance before the logical viewport resize it renders.
  // Paginated zoom does not reflow, so it can keep following the requested render zoom directly.
  val continuous = layoutSpec is EditorDocumentLayoutSpec.Continuous
  var committedContinuousRenderZoom by remember(editor, continuous) { mutableFloatStateOf(1f) }
  val continuousRenderZoom =
    if (continuous) {
      requestedRenderZoom.takeIf { it.isFinite() && it > 0f } ?: 1f
    } else {
      1f
    }
  val localEditAdmissionGeneration =
    if (continuous && editor != null) {
      editor.localEditAdmissionGeneration.collectAsState().value
    } else {
      0L
    }

  LaunchedEffect(
    editor,
    physicalViewport,
    layoutSpec,
    continuousRenderZoom,
    scaleFactor,
    localEditAdmissionGeneration,
  ) {
    val activeEditor = editor ?: return@LaunchedEffect
    if (
      physicalViewport.width <= 0f ||
        physicalViewport.height <= 0f ||
        !scaleFactor.isFinite() ||
        scaleFactor <= 0.0
    ) {
      return@LaunchedEffect
    }
    var resizeCommitted = false
    val resizeEffectCompleted = activeEditor.runEffect {
      resizeCommitted =
        activeEditor.update {
          enqueue(
            Message.System(
              SystemEvent.Resize(
                width =
                  when (layoutSpec) {
                    is EditorDocumentLayoutSpec.Continuous ->
                      resolveContinuousLayoutViewportWidth(
                        viewportWidth = physicalViewport.width,
                        committedZoom = continuousRenderZoom,
                      )
                    is EditorDocumentLayoutSpec.Paginated -> physicalViewport.width
                  },
                height = physicalViewport.height,
                scaleFactor = scaleFactor,
              )
            )
          )
        } != null
    }
    if (resizeEffectCompleted && resizeCommitted && continuous) {
      committedContinuousRenderZoom = continuousRenderZoom
    }
  }

  return if (continuous) {
    committedContinuousRenderZoom
  } else {
    requestedRenderZoom
  }
}
