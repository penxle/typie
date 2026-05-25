package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.Dp
import co.typie.screen.editor.editor.toolbar.bottom.BottomToolbarNodes
import co.typie.screen.editor.editor.toolbar.bottom.BottomToolbarTools
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect

@Composable
internal fun BottomToolbar(
  panel: EditorToolbarBottomPanelKey,
  height: Dp,
  onEditorInputRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val hazeState = LocalHazeState.current
  val surfaceColor = AppTheme.colors.surfaceCanvas
  val borderColor = AppTheme.colors.borderDefault

  Box(
    modifier =
      modifier
        .fillMaxWidth()
        .height(height)
        .shadow(AppTheme.shadows.sm, ToolbarBottomPanelShape)
        .clip(ToolbarBottomPanelShape)
        .hazeEffect(hazeState) {
          blurEffect {
            backgroundColor = surfaceColor
            blurRadius = ToolbarBackdropBlurRadius
          }
        }
        .background(surfaceColor.copy(alpha = BottomToolbarSurfaceAlpha), ToolbarBottomPanelShape)
        .border(
          ToolbarBorderWidth,
          borderColor.copy(alpha = BottomToolbarBorderAlpha),
          ToolbarBottomPanelShape,
        )
  ) {
    when (panel) {
      EditorToolbarBottomPanelKey.Insert ->
        BottomToolbarNodes(
          onEditorInputRequest = onEditorInputRequest,
          modifier = Modifier.fillMaxSize(),
        )
      EditorToolbarBottomPanelKey.Tools -> BottomToolbarTools(modifier = Modifier.fillMaxSize())
    }
  }
}

private const val BottomToolbarSurfaceAlpha = 0.86f
private const val BottomToolbarBorderAlpha = 0.55f
