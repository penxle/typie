package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.horizontalScroll
import co.typie.ext.verticalScroll
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.max

private enum class EditorScreenLayoutSlot {
  ScrollContent,
  ViewportOverlay,
  Overlay,
  Toolbar,
}

@Composable
internal fun EditorScreenLayout(
  state: EditorScreenState,
  horizontalScrollEnabled: Boolean,
  horizontalScrollContentWidth: Float,
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
    val sharedTrackWidth = max(state.viewport.width, horizontalScrollContentWidth)
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
    val scrollContentPlaceables =
      subcompose(EditorScreenLayoutSlot.ScrollContent) {
          Box(
            modifier =
              Modifier.fillMaxSize().onSizeChanged { size ->
                state.updateViewport(resolveSize(size.width, size.height))
              }
          ) {
            Column(Modifier.fillMaxSize().verticalScroll(state.scrollState)) {
              if (horizontalScrollEnabled) {
                Box(
                  modifier = Modifier.fillMaxWidth().horizontalScroll(state.horizontalScrollState)
                ) {
                  Column(
                    modifier =
                      Modifier.run {
                        if (sharedTrackWidth > 0f) {
                          width(sharedTrackWidth.dp)
                        } else {
                          fillMaxWidth()
                        }
                      }
                  ) {
                    header()
                    body()
                  }
                }
              } else {
                header()
                body()
              }
            }
          }
        }
        .map { it.measure(viewportConstraints) }
    val viewportOverlayPlaceables =
      subcompose(EditorScreenLayoutSlot.ViewportOverlay) {
          Box(modifier = Modifier.fillMaxSize(), content = viewportOverlay)
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
      scrollContentPlaceables.forEach { it.place(x = 0, y = 0) }
      viewportOverlayPlaceables.forEach { it.place(x = 0, y = 0) }
      overlayPlaceables.forEach { it.place(x = 0, y = 0) }
      toolbarPlaceables.forEach { it.place(x = 0, y = constraints.maxHeight - it.height) }
    }
  }
}
