package co.typie.editor.surface

import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.render.RenderCanvas
import co.typie.editor.runtime.LocalEditorRuntime
import kotlin.math.round

private val DebugRustSurfaceTint = Color(0x220096FF)

@Composable
internal fun EditorPageSurface(
  page: Int,
  width: Float,
  height: Float,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val scaleFactor = density.density.toDouble()
  val editor = LocalEditorRuntime.current.editor ?: return

  val widthDouble = width.toDouble()
  val heightDouble = height.toDouble()
  val canvasWidth = round(widthDouble * scaleFactor)
  val canvasHeight = round(heightDouble * scaleFactor)
  val quantizedWidthDp = Dp((canvasWidth / scaleFactor).toFloat())
  val quantizedHeightDp = Dp((canvasHeight / scaleFactor).toFloat())

  RenderCanvas(
    modifier =
      modifier.width(quantizedWidthDp).height(quantizedHeightDp).drawWithContent {
        drawContent()
        drawRect(DebugRustSurfaceTint)
      },
    onAttach = { handle ->
      editor.attachSurface(page, handle, widthDouble, heightDouble, scaleFactor)
      editor.renderSurface(page)
    },
    onDetach = { editor.detachSurface(page) },
    onResize = {
      editor.resizeSurface(page, widthDouble, heightDouble, scaleFactor)
      editor.renderSurface(page)
    },
  )

  DisposableEffect(editor, page) {
    val off = editor.on<EditorEvent.RenderInvalidated> { ed, _ -> ed.renderSurface(page) }

    onDispose { off() }
  }
}
