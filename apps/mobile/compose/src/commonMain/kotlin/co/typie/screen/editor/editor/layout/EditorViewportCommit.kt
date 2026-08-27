package co.typie.screen.editor.editor.layout

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import co.typie.editor.Editor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport

@Composable
internal fun rememberCommittedEditorRenderZoom(
  editor: Editor?,
  viewport: Viewport?,
  layoutSpec: EditorDocumentLayoutSpec,
  requestedRenderZoom: Float,
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

  LaunchedEffect(editor, viewport, continuous, continuousRenderZoom, localEditAdmissionGeneration) {
    val activeEditor = editor ?: return@LaunchedEffect
    val targetViewport = viewport ?: return@LaunchedEffect
    var resizeCommitted = false
    val resizeEffectCompleted = activeEditor.runEffect {
      resizeCommitted =
        activeEditor.update {
          enqueue(
            Message.System(
              SystemEvent.Resize(
                width = targetViewport.width,
                height = targetViewport.height,
                scaleFactor = targetViewport.scaleFactor,
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
