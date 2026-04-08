package co.typie.editor.compose

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import co.typie.editor.Editor
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

  val lastRendered = remember { floatArrayOf(width, height) }

  Box(modifier = Modifier.background(AppTheme.colors.surfaceDefault)) {
    Surface(
      modifier = modifier.width(Dp(width)).height(Dp(height)),
      onAttach = { handle ->
        editor.attachSurface(page, handle, width.toInt(), height.toInt(), scaleFactor)
        editor.renderSurface(page)
      },
      onDetach = {
        editor.detachSurface(page)
      },
    )
  }

  DisposableEffect(editor, page) {
    val off = editor.on<EditorEvent.RenderInvalidated> { ed, _ ->
      ed.renderSurface(page)
    }

    onDispose { off() }
  }

  SideEffect {
    if (lastRendered[0] != width || lastRendered[1] != height) {
      editor.resizeSurface(page, width.toInt(), height.toInt(), scaleFactor)
      editor.renderSurface(page)
      lastRendered[0] = width
      lastRendered[1] = height
    }
  }
}
