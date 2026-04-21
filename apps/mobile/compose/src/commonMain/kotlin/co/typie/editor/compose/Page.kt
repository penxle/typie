package co.typie.editor.compose

import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import co.typie.editor.Editor
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.render.RenderCanvas
import kotlin.math.round

@Composable
internal fun Page(
  editor: Editor,
  page: Int,
  width: Float,
  height: Float,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val scaleFactor = density.density.toDouble()

  val widthDouble = width.toDouble()
  val heightDouble = height.toDouble()
  val canvasWidth = round(widthDouble * scaleFactor)
  val canvasHeight = round(heightDouble * scaleFactor)
  val quantizedWidthDp = Dp((canvasWidth / scaleFactor).toFloat())
  val quantizedHeightDp = Dp((canvasHeight / scaleFactor).toFloat())

  RenderCanvas(
    modifier = modifier.width(quantizedWidthDp).height(quantizedHeightDp),
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
