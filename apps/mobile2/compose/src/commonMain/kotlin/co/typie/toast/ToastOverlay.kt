package co.typie.toast

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseIn
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
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
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun ToastOverlay() {
  val toast = koinInject<Toast>()
  val toastState by toast.state.collectAsState()

  val density = LocalDensity.current
  val imeBottom = WindowInsets.ime.getBottom(density)
  val safeBottom = WindowInsets.safeDrawing.getBottom(density)
  val bottomOffsetPx = imeBottom + safeBottom + with(density) { 12.dp.toPx() }.toInt()

  Box(Modifier.fillMaxSize()) {
    toastState?.let { state ->
      AnimatedToast(
        state = state,
        bottomOffsetPx = bottomOffsetPx,
        onDismiss = { toast.dismiss() },
      )
    }
  }
}

@Composable
private fun AnimatedToast(
  state: ToastState,
  bottomOffsetPx: Int,
  onDismiss: () -> Unit,
) {
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
    modifier = Modifier
      .fillMaxSize()
      .alpha(alpha.value)
      .graphicsLayer {
        translationY = slideOffset.value * 0.2f * size.height
      },
    contentAlignment = Alignment.BottomCenter,
  ) {
    Box(
      modifier = Modifier
        .offset { IntOffset(0, -bottomOffsetPx) }
        .padding(horizontal = 24.dp)
        .fillMaxWidth()
        .background(AppTheme.colors.surfaceDark, RoundedCornerShape(22.dp))
        .padding(12.dp),
    ) {
      Row(verticalAlignment = Alignment.Top) {
        Box(
          modifier = Modifier
            .padding(top = 1.dp)
            .size(20.dp)
            .background(
              when (state.type) {
                ToastType.Success -> AppTheme.colors.accentSuccess
                ToastType.Error -> AppTheme.colors.accentDanger
                ToastType.Notification -> AppTheme.colors.accentSuccess
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
            tint = AppTheme.colors.textBright,
            modifier = Modifier.size(12.dp),
          )
        }
        Spacer(Modifier.width(8.dp))
        Text(
          text = state.message,
          style = TextStyle(
            fontSize = 14.sp,
            fontWeight = FontWeight.W500,
            color = AppTheme.colors.textBright,
          ),
        )
      }
    }
  }
}
