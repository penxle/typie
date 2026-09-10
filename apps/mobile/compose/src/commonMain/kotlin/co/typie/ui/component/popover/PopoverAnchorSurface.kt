package co.typie.ui.component.popover

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.rememberPressScale
import co.typie.ui.theme.AppShadow
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.shadow

internal data class PopoverAnchorSurface(
  val background: Color,
  val border: Color,
  val cornerRadius: Dp?,
  val shadow: AppShadow,
  val scale: State<Float>?,
)

internal class PopoverAnchorSurfaceState {
  var surface: PopoverAnchorSurface? by mutableStateOf(null)
}

internal val LocalPopoverAnchorSurfaceState =
  staticCompositionLocalOf<PopoverAnchorSurfaceState?> { null }

internal val LocalPopoverAnchorContentOnly = staticCompositionLocalOf { false }

/** Shares the trigger's surface with the popover while its icon or label crossfades separately. */
@Composable
internal fun Modifier.popoverAnchorSurface(
  background: Color,
  border: Color,
  cornerRadius: Dp? = null,
  shadow: AppShadow = AppShadow.None,
  pressedScale: Float? = null,
): Modifier {
  if (LocalPopoverAnchorContentOnly.current) return this

  val state = LocalPopoverAnchorSurfaceState.current
  val interactionSource = LocalInteractionSource.current
  val scale =
    if (pressedScale != null && interactionSource != null)
      rememberPressScale(interactionSource, pressedScale)
    else null
  if (state != null) {
    SideEffect {
      state.surface = PopoverAnchorSurface(background, border, cornerRadius, shadow, scale)
    }
  }
  val shape = cornerRadius?.let(AppShapes::rounded) ?: AppShapes.circle
  return shadow(shadow, shape)
    .then(
      if (scale != null)
        Modifier.graphicsLayer {
          scaleX = scale.value
          scaleY = scale.value
        }
      else Modifier
    )
    .background(background, shape)
    .border(1.dp, border, shape)
}
