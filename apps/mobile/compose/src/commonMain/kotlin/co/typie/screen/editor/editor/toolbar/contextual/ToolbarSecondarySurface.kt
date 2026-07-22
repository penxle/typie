package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.BringIntoViewSpec
import androidx.compose.foundation.gestures.LocalBringIntoViewSpec
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalDensity
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
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryContentRevealPadding
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryContentStartInset
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryHeight
import co.typie.screen.editor.editor.toolbar.preserveEditorFocusOnToolbarInteraction
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect
import kotlin.math.abs

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun ToolbarSecondarySurface(
  onClose: () -> Unit,
  closeContentDescription: String,
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val hazeState = LocalHazeState.current
  val toolbarSurfaceColor = AppTheme.colors.surfaceDefault
  val density = LocalDensity.current
  val bringIntoViewSpec =
    remember(density) {
      ToolbarSecondaryBringIntoViewSpec(
        startInset =
          with(density) {
            (ToolbarSecondaryContentStartInset + ToolbarSecondaryContentRevealPadding).toPx()
          },
        endInset = with(density) { ToolbarSecondaryContentRevealPadding.toPx() },
      )
    }

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
    CompositionLocalProvider(LocalBringIntoViewSpec provides bringIntoViewSpec) { content() }
    Box(modifier = Modifier.align(Alignment.CenterStart)) {
      ToolbarSecondaryCloseButton(contentDescription = closeContentDescription, onClick = onClose)
    }
  }
}

private class ToolbarSecondaryBringIntoViewSpec(
  private val startInset: Float,
  private val endInset: Float,
) : BringIntoViewSpec {
  override fun calculateScrollDistance(offset: Float, size: Float, containerSize: Float): Float {
    val leadingEdge = offset - startInset
    val adjustedContainerSize = (containerSize - startInset - endInset).coerceAtLeast(0f)
    val trailingEdge = leadingEdge + size

    return when {
      leadingEdge >= 0f && trailingEdge <= adjustedContainerSize -> 0f
      leadingEdge < 0f && trailingEdge > adjustedContainerSize -> 0f
      abs(leadingEdge) < abs(trailingEdge - adjustedContainerSize) -> leadingEdge
      else -> trailingEdge - adjustedContainerSize
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
