package co.typie.ui.component.topbar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import co.typie.ext.statusBars
import co.typie.ext.toDp

@Composable
fun TopBar(
  modifier: Modifier = Modifier,
  leading: (@Composable () -> Unit)? = null,
  center: (@Composable () -> Unit)? = null,
  trailing: (@Composable () -> Unit)? = null,
  visible: Boolean = true,
  scrollOffset: (() -> Int)? = null,
  onTap: (() -> Unit)? = null,
) {
  val density = LocalDensity.current
  val centerRevealed by remember(scrollOffset, density) {
    derivedStateOf {
      val offsetDp = (scrollOffset?.invoke() ?: Int.MAX_VALUE).toDp(density)
      offsetDp > TopBarDefaults.RevealOffset
    }
  }

  val visibilityAlpha by animateFloatAsState(
    targetValue = if (visible) 1f else 0f,
    animationSpec = tween(TopBarDefaults.VisibilityFadeDuration),
  )
  val visibilityOffsetY by animateFloatAsState(
    targetValue = if (visible) 0f else -1f,
    animationSpec = tween(
      TopBarDefaults.VisibilityAnimationDuration,
      easing = EaseOutCubic,
    ),
  )

  Box(
    modifier = modifier
      .fillMaxWidth()
      .graphicsLayer {
        alpha = visibilityAlpha
        translationY = visibilityOffsetY * size.height
      }
      .windowInsetsPadding(WindowInsets.statusBars)
      .then(
        if (onTap != null) Modifier.pointerInput(onTap) {
          detectTapGestures { onTap() }
        } else Modifier
      ),
  ) {
    Row(
      verticalAlignment = Alignment.CenterVertically,
      modifier = Modifier
        .fillMaxWidth()
        .height(TopBarDefaults.Height)
        .padding(horizontal = TopBarDefaults.HorizontalPadding),
    ) {
      // Leading slot
      Box(
        contentAlignment = Alignment.CenterStart,
        modifier = Modifier
          .width(TopBarDefaults.SlotWidth)
          .height(TopBarDefaults.Height),
      ) {
        leading?.invoke()
      }

      Spacer(Modifier.width(TopBarDefaults.SlotGap))

      // Center slot with reveal animation
      Box(
        contentAlignment = Alignment.Center,
        modifier = Modifier
          .weight(1f)
          .height(TopBarDefaults.Height),
      ) {
        TopBarCenterReveal(visible = centerRevealed, content = center)
      }

      Spacer(Modifier.width(TopBarDefaults.SlotGap))

      // Trailing slot
      Box(
        contentAlignment = Alignment.CenterEnd,
        modifier = Modifier
          .width(TopBarDefaults.SlotWidth)
          .height(TopBarDefaults.Height),
      ) {
        trailing?.invoke()
      }
    }
  }
}

@Composable
private fun TopBarCenterReveal(
  visible: Boolean,
  content: (@Composable () -> Unit)?,
) {
  AnimatedVisibility(
    visible = visible,
    enter = fadeIn(tween(TopBarDefaults.RevealFadeDuration)) +
      slideInVertically(
        animationSpec = tween(
          TopBarDefaults.RevealAnimationDuration,
          easing = EaseOut,
        ),
        initialOffsetY = { (it * 0.4f).toInt() },
      ),
    exit = fadeOut(tween(TopBarDefaults.RevealFadeDuration)) +
      slideOutVertically(
        animationSpec = tween(
          TopBarDefaults.RevealAnimationDuration,
          easing = EaseOut,
        ),
        targetOffsetY = { (it * 0.4f).toInt() },
      ),
  ) {
    content?.invoke()
  }
}

fun ScrollState.topBarScrollOffset(): () -> Int = { value }

fun LazyListState.topBarScrollOffset(): () -> Int = {
  if (firstVisibleItemIndex > 0) Int.MAX_VALUE
  else firstVisibleItemScrollOffset
}
