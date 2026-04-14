package co.typie.domain.editor.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import co.typie.domain.editor.Editor
import co.typie.editor.ffi.EditorEvent
import co.typie.ui.theme.AppTheme

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

  Box(modifier = Modifier.background(AppTheme.colors.surfaceDefault)) {
    Surface(
      modifier = modifier.width(Dp(width)).height(Dp(height)),
      onAttach = { handle ->
        editor.attachSurface(page, handle, width.toInt(), height.toInt(), scaleFactor)
        editor.renderSurface(page)
      },
      onDetach = { editor.detachSurface(page) },
      onResize = {
        editor.resizeSurface(page, width.toInt(), height.toInt(), scaleFactor)
        editor.renderSurface(page)
      },
    )
  }

  DisposableEffect(editor, page) {
    val off = editor.on<EditorEvent.RenderInvalidated> { ed, _ -> ed.renderSurface(page) }

    onDispose { off() }
  }
}
