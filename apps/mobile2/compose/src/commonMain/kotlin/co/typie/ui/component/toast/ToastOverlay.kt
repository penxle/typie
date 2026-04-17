package co.typie.ui.component.toast

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.Crossfade
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseIn
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Canvas
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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.safeDrawing
import co.typie.ext.toDp
import co.typie.ext.toPx
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.hazeEffect
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun ToastOverlay() {
  val toast = LocalToast.current
  val toastState = toast.state

  val density = LocalDensity.current
  val safeDrawingBottom = WindowInsets.safeDrawing.getBottom(density).toDp(density)

  var rootHeight by remember { mutableStateOf(0f) }
  val fallbackY = rootHeight - with(density) { (safeDrawingBottom + 12.dp).toPx() }
  val targetY = toast.anchorY ?: fallbackY

  val animatedAnchorY by
    animateFloatAsState(
      targetY,
      spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessMediumLow),
    )
  val bottomOffset = with(density) { (rootHeight - animatedAnchorY).toDp() } + 12.dp

  var visibleState by remember { mutableStateOf<ToastState?>(null) }

  if (toastState != null) {
    visibleState = toastState
  }

  Box(Modifier.fillMaxSize().onSizeChanged { rootHeight = it.height.toFloat() }) {
    visibleState?.let { state ->
      AnimatedToast(
        state = state,
        bottomOffset = bottomOffset,
        dismissed = toastState == null,
        onDismiss = {
          visibleState = null
          toast.dismiss()
        },
      )
    }
  }
}

@Composable
private fun AnimatedToast(
  state: ToastState,
  bottomOffset: Dp,
  dismissed: Boolean,
  onDismiss: () -> Unit,
) {
  AppTheme.colors
  val toastSurface =
    if (
      when (AppTheme.themeMode) {
        ResolvedThemeMode.Light -> false
        ResolvedThemeMode.Dark -> true
      }
    )
      AppColor.dark.gray.s500
    else AppColor.light.gray.s600
  val toastText = AppColor.white
  val density = LocalDensity.current
  val alpha = remember { Animatable(0f) }
  val slideOffset = remember { Animatable(1f) }

  LaunchedEffect(state.id) {
    alpha.snapTo(0f)
    slideOffset.snapTo(1f)
    coroutineScope {
      launch { alpha.animateTo(1f, tween(200, easing = EaseOut)) }
      launch { slideOffset.animateTo(0f, tween(200, easing = EaseOut)) }
    }
  }

  LaunchedEffect(state.id, state.type) {
    if (state.type != ToastType.Loading) {
      delay(state.duration.inWholeMilliseconds)
      coroutineScope {
        launch { alpha.animateTo(0f, tween(200, easing = EaseIn)) }
        launch { slideOffset.animateTo(1f, tween(200, easing = EaseIn)) }
      }
      onDismiss()
    }
  }

  LaunchedEffect(dismissed) {
    if (dismissed) {
      coroutineScope {
        launch { alpha.animateTo(0f, tween(200, easing = EaseIn)) }
        launch { slideOffset.animateTo(1f, tween(200, easing = EaseIn)) }
      }
      onDismiss()
    }
  }

  Box(
    modifier =
      Modifier.fillMaxSize().graphicsLayer {
        this.alpha = alpha.value
        translationY = slideOffset.value * 4.dp.toPx(density)
      },
    contentAlignment = Alignment.BottomCenter,
  ) {
    Box(
      modifier =
        Modifier.offset(y = -bottomOffset)
          .padding(horizontal = 16.dp)
          .fillMaxWidth()
          .clip(AppShapes.rounded(AppShapes.lg))
          .hazeEffect(LocalHazeState.current)
          .background(toastSurface.copy(alpha = .6f))
          .padding(horizontal = 24.dp, vertical = 16.dp)
    ) {
      Row(verticalAlignment = Alignment.CenterVertically) {
        Crossfade(targetState = state.type, animationSpec = tween(200)) { type ->
          when (type) {
            ToastType.Loading -> ToastSpinner()
            else -> {
              Box(
                modifier =
                  Modifier.size(20.dp)
                    .background(
                      when (type) {
                        ToastType.Success -> AppTheme.colors.success
                        ToastType.Error -> AppTheme.colors.danger
                        else -> AppTheme.colors.brand
                      },
                      AppShapes.circle,
                    ),
                contentAlignment = Alignment.Center,
              ) {
                Icon(
                  icon =
                    when (type) {
                      ToastType.Success -> Lucide.Check
                      ToastType.Error -> Typie.Exclamation
                      else -> Lucide.Bell
                    },
                  strokeWidth =
                    when (type) {
                      ToastType.Success -> 4f
                      ToastType.Error -> 1.75f
                      else -> 2.5f
                    },
                  tint = toastText,
                  modifier = Modifier.size(12.dp),
                )
              }
            }
          }
        }
        Spacer(Modifier.width(8.dp))
        AnimatedContent(
          targetState = state.message,
          transitionSpec = {
            (slideInVertically { it } + fadeIn(tween(200))) togetherWith
              (slideOutVertically { -it } + fadeOut(tween(200))) using
              SizeTransform(clip = false)
          },
        ) { message ->
          Text(text = message, style = AppTheme.typography.caption, color = toastText)
        }
      }
    }
  }
}

@Composable
private fun ToastSpinner() {
  val color = AppColor.white
  val transition = rememberInfiniteTransition()
  val rotation by
    transition.animateFloat(
      initialValue = 0f,
      targetValue = 360f,
      animationSpec =
        infiniteRepeatable(
          animation = tween(1000, easing = LinearEasing),
          repeatMode = RepeatMode.Restart,
        ),
    )
  Box(Modifier.size(20.dp), contentAlignment = Alignment.Center) {
    Canvas(Modifier.size(14.dp)) {
      drawArc(
        color = color,
        startAngle = rotation,
        sweepAngle = 270f,
        useCenter = false,
        style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round),
      )
    }
  }
}
