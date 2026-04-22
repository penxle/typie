package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import co.typie.ext.verticalScroll
import co.typie.screen.editor.editor.state.EditorScreenState

private enum class EditorScreenLayoutSlot {
  ScrollContent,
  Overlay,
  Toolbar,
}

@Composable
internal fun EditorScreenLayout(
  state: EditorScreenState,
  header: @Composable () -> Unit,
  body: @Composable () -> Unit,
  overlay: @Composable () -> Unit = {},
  toolbar: @Composable () -> Unit,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val resolveSize: (Int, Int) -> Size =
    remember(density) {
      { width, height -> Size(width = width / density.density, height = height / density.density) }
    }

  SubcomposeLayout(
    modifier =
      modifier.fillMaxSize().onSizeChanged { size ->
        state.updateViewport(resolveSize(size.width, size.height))
      }
  ) { constraints ->
    val scrollContentPlaceables =
      subcompose(EditorScreenLayoutSlot.ScrollContent) {
          Column(Modifier.fillMaxSize().verticalScroll(state.scrollState)) {
            header()
            body()
          }
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
    val toolbarPlaceables =
      subcompose(EditorScreenLayoutSlot.Toolbar, toolbar).map {
        it.measure(constraints.copy(minWidth = 0, minHeight = 0))
      }
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
      scrollContentPlaceables.forEach { it.place(x = 0, y = 0) }
      overlayPlaceables.forEach { it.place(x = 0, y = 0) }
      toolbarPlaceables.forEach { it.place(x = 0, y = constraints.maxHeight - it.height) }
    }
  }
}
