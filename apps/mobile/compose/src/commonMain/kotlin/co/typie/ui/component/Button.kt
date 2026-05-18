package co.typie.ui.component

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppColors
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay

enum class ButtonVariant {
  Primary,
  Secondary,
  Danger,
}

@Immutable private data class ButtonColors(val background: Color, val text: Color)

private fun AppColors.buttonColors(variant: ButtonVariant): ButtonColors =
  when (variant) {
    ButtonVariant.Primary -> ButtonColors(background = textDefault, text = surfaceCanvas)
    ButtonVariant.Secondary -> ButtonColors(background = surfaceInset, text = textDefault)
    ButtonVariant.Danger -> ButtonColors(background = danger, text = textOnDanger)
  }

@Composable
fun Button(
  text: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  leading: (@Composable (Color) -> Unit)? = null,
  trailing: (@Composable (Color) -> Unit)? = null,
  loadingText: String? = null,
  variant: ButtonVariant = ButtonVariant.Primary,
  enabled: Boolean = true,
  loading: Boolean = false,
  height: Dp = 48.dp,
  textStyle: TextStyle? = null,
) {
  var debouncedLoading by remember { mutableStateOf(false) }
  LaunchedEffect(loading) {
    if (loading) {
      delay(300)
      debouncedLoading = true
    } else {
      debouncedLoading = false
    }
  }

  val colors = AppTheme.colors.buttonColors(variant)
  val interactive = enabled && !loading
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)

  Skeleton.Bone(
    modifier = modifier.fillMaxWidth().height(height),
    shape = AppShapes.rounded(AppShapes.lg),
  ) {
    InteractionScope {
      Box(
        modifier =
          modifier
            .fillMaxWidth()
            .height(height)
            .graphicsLayer { this.alpha = alpha }
            .background(colors.background, AppShapes.rounded(AppShapes.lg))
            .clickable(enabled = interactive, onClick = onClick),
        contentAlignment = Alignment.Center,
      ) {
        val spinnerAlpha by animateFloatAsState(if (debouncedLoading) 1f else 0f, tween(150))
        val spinnerWidth = if (debouncedLoading) 16.dp else 0.dp

        Row(modifier = Modifier.pressScale(0.95f), verticalAlignment = Alignment.CenterVertically) {
          Box(modifier = Modifier.width(spinnerWidth), contentAlignment = Alignment.CenterStart) {
            if (debouncedLoading) {
              Spinner(
                color = colors.text,
                modifier = Modifier.graphicsLayer { this.alpha = spinnerAlpha },
              )
            }
          }

          if (debouncedLoading) {
            Spacer(Modifier.width(10.dp))
          }

          if (leading != null) {
            Box(modifier = Modifier.size(16.dp), contentAlignment = Alignment.Center) {
              leading(colors.text)
            }

            Spacer(Modifier.width(8.dp))
          }

          val displayText = if (debouncedLoading && loadingText != null) loadingText else text
          AnimatedContent(
            targetState = displayText,
            transitionSpec = { fadeIn(tween(150)) togetherWith fadeOut(tween(150)) },
          ) { label ->
            Text(
              text = label,
              style = AppTheme.typography.action.merge(textStyle),
              color = colors.text,
            )
          }

          if (trailing != null) {
            Spacer(Modifier.width(8.dp))

            Box(modifier = Modifier.size(16.dp), contentAlignment = Alignment.Center) {
              trailing(colors.text)
            }
          }
        }
      }
    }
  }
}
