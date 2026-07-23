package co.typie.editor.overlay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.trackEditorInteractionSurfaceBounds
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import co.typie.editor.runtime.EditorUiState
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class LineHighlightDesktopTest {
  @Test
  fun continuousLineHighlightSurvivesContentTallerThanConstraintsLimit() = runComposeUiTest {
    val uiState = EditorUiState()
    val cursor =
      CursorMetrics(
        pageIdx = 0,
        caret = Rect(x = 0f, y = 10f, width = 1f, height = 10f),
        line = Rect(x = 0f, y = 10f, width = 100f, height = 10f),
      )

    setContent {
      CompositionLocalProvider(LocalDensity provides Density(1f)) {
        Layout(
          content = {
            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .trackEditorInteractionSurfaceBounds(uiState = uiState, density = 1f)
                  .editorExtensionAreaLineHighlight(
                    cursor = cursor,
                    focused = true,
                    editorBounds = { uiState.editorBoundsInContainer },
                    viewportTransform = { uiState.resolveViewportTransform() },
                    enabled = true,
                    color = Color.Black,
                  )
            ) {
              Column {
                repeat(ContentBlockCount) {
                  Box(modifier = Modifier.fillMaxWidth().height(ContentBlockHeight.dp))
                }
              }
            }
          }
        ) { measurables, constraints ->
          val placeable =
            measurables
              .single()
              .measure(
                Constraints(
                  minWidth = constraints.maxWidth,
                  maxWidth = constraints.maxWidth,
                  minHeight = 0,
                  maxHeight = Constraints.Infinity,
                )
              )
          layout(constraints.maxWidth, constraints.maxHeight) { placeable.place(0, 0) }
        }
      }
    }
  }

  private companion object {
    const val ContentBlockCount = 2
    const val ContentBlockHeight = 150_000f
  }
}
