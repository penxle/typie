package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarIconButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarSurfaceBackground
import co.typie.screen.editor.editor.toolbar.ToolbarBackdropBlurRadius
import co.typie.screen.editor.editor.toolbar.ToolbarBorderWidth
import co.typie.screen.editor.editor.toolbar.ToolbarCapsuleShape
import co.typie.screen.editor.editor.toolbar.ToolbarFixedActionPadding
import co.typie.screen.editor.editor.toolbar.ToolbarFixedActionShape
import co.typie.screen.editor.editor.toolbar.ToolbarFixedActionWidth
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryHeight
import co.typie.screen.editor.editor.toolbar.preserveEditorFocusOnToolbarInteraction
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect

@Composable
internal fun ToolbarSecondarySurface(
  onClose: () -> Unit,
  closeContentDescription: String,
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val hazeState = LocalHazeState.current
  val toolbarSurfaceColor = AppTheme.colors.surfaceDefault

  Box(
    modifier =
      modifier
        .fillMaxWidth()
        .height(ToolbarSecondaryHeight)
        .shadow(AppTheme.shadows.sm, ToolbarCapsuleShape)
        .clip(ToolbarCapsuleShape)
        .hazeEffect(hazeState) {
          blurEffect {
            backgroundColor = toolbarSurfaceColor
            blurRadius = ToolbarBackdropBlurRadius
          }
        }
        .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarCapsuleShape)
        .preserveEditorFocusOnToolbarInteraction()
  ) {
    EditorToolbarSurfaceBackground(shape = ToolbarCapsuleShape)
    content()
    Box(modifier = Modifier.align(Alignment.CenterStart)) {
      ToolbarSecondaryCloseButton(contentDescription = closeContentDescription, onClick = onClose)
    }
  }
}

@Composable
private fun ToolbarSecondaryCloseButton(contentDescription: String, onClick: () -> Unit) {
  InteractionScope {
    EditorToolbarIconButton(
      icon = Lucide.X,
      contentDescription = contentDescription,
      onClick = onClick,
      shape = ToolbarFixedActionShape,
      fixedActionSurface = true,
      inheritInteractionSource = true,
      modifier =
        Modifier.width(ToolbarFixedActionWidth).fillMaxHeight().padding(ToolbarFixedActionPadding),
      iconSize = 20.dp,
    )
  }
}
