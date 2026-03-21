package co.typie.overlay

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseIn
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ext.safeDrawing
import co.typie.ext.toDp
import co.typie.ext.toPx
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeEffect
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun ToastOverlay() {
  val toast = koinInject<Toast>()
  val toastState by toast.state.collectAsState()

  val density = LocalDensity.current
  val imeBottom = WindowInsets.ime.getBottom(density).toDp(density)
  val safeBottom = WindowInsets.safeDrawing.getBottom(density).toDp(density)
  val animatedBottomInset by animateDpAsState(
    toast.bottomInset,
    spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMediumLow)
  )
  val bottomOffset = imeBottom + safeBottom + 12.dp + animatedBottomInset

  Box(Modifier.fillMaxSize()) {
    toastState?.let { state ->
      AnimatedToast(
        state = state,
        bottomOffset = bottomOffset,
        onDismiss = { toast.dismiss() },
      )
    }
  }
}

@Composable
private fun AnimatedToast(
  state: ToastState,
  bottomOffset: Dp,
  onDismiss: () -> Unit,
) {
  val colors = AppTheme.colors
  val density = LocalDensity.current
  val alpha = remember { Animatable(0f) }
  val slideOffset = remember { Animatable(1f) }

  LaunchedEffect(state) {
    // Enter (simultaneous fade + slide)
    alpha.snapTo(0f)
    slideOffset.snapTo(1f)
    coroutineScope {
      launch { alpha.animateTo(1f, tween(200, easing = EaseOut)) }
      launch { slideOffset.animateTo(0f, tween(200, easing = EaseOut)) }
    }

    // Wait
    delay(state.duration.inWholeMilliseconds)

    // Exit (simultaneous fade + slide)
    coroutineScope {
      launch { alpha.animateTo(0f, tween(200, easing = EaseIn)) }
      launch { slideOffset.animateTo(1f, tween(200, easing = EaseIn)) }
    }

    onDismiss()
  }

  Box(
    modifier = Modifier.fillMaxSize().alpha(alpha.value).graphicsLayer {
      translationY = slideOffset.value * 4.dp.toPx(density)
    },
    contentAlignment = Alignment.BottomCenter,
  ) {
    Box(
      modifier = Modifier.offset(y = -bottomOffset).padding(horizontal = 16.dp).fillMaxWidth()
        .clip(RoundedCornerShape(16.dp)).hazeEffect(LocalHazeState.current)
        .background(colors.surfaceDark.copy(alpha = .6f))
        .padding(horizontal = 24.dp, vertical = 16.dp),
    ) {
      Row(verticalAlignment = Alignment.Top) {
        Box(
          modifier = Modifier.size(20.dp).background(
            when (state.type) {
              ToastType.Success -> AppTheme.colors.accentSuccess
              ToastType.Error -> AppTheme.colors.accentDanger
              ToastType.Notification -> AppTheme.colors.accentBrand
            },
            CircleShape,
          ),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = when (state.type) {
              ToastType.Success -> Lucide.Check
              ToastType.Error -> Typie.ExclamationSvg
              ToastType.Notification -> Lucide.Bell
            },
            strokeWidth = when (state.type) {
              ToastType.Success -> 4f
              ToastType.Error -> 1.75f
              ToastType.Notification -> 2.5f
            },
            tint = AppTheme.colors.textBright,
            modifier = Modifier.size(12.dp),
          )
        }
        Spacer(Modifier.width(8.dp))
        Text(
          text = state.message,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textBright,
          modifier = Modifier.align(Alignment.CenterVertically),
        )
      }
    }
  }
}
