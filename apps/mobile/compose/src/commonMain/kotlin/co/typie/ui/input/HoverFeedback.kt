package co.typie.ui.input

import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import co.typie.ext.LocalInteractionSource
import co.typie.ui.theme.AppTheme

/** Apply after the control's resting background; foreground content is never tinted. */
@Composable
fun Modifier.hoverFeedback(
  interactionSource: InteractionSource = checkNotNull(LocalInteractionSource.current),
  enabled: Boolean = true,
  shape: Shape,
  hoverColor: Color = AppTheme.colors.surfaceHover,
  activeColor: Color = AppTheme.colors.surfaceActive,
): Modifier {
  val hovered by interactionSource.collectIsHoveredAsState()
  val pressed by interactionSource.collectIsPressedAsState()
  return background(
    if (enabled && hovered) {
      if (pressed) activeColor else hoverColor
    } else Color.Transparent,
    shape,
  )
}
