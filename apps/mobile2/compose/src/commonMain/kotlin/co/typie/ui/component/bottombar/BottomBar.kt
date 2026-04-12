package co.typie.ui.component.bottombar

import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.pointerIgnore
import co.typie.ext.toPx

@Composable
fun BottomBar(state: BottomBarState, modifier: Modifier = Modifier) {
  val density = LocalDensity.current
  val alpha by
    animateFloatAsState(targetValue = if (state.enabled) 1f else 0f, animationSpec = tween(200))
  val maxTranslation = 120.dp.toPx(density)
  val translationY by
    animateFloatAsState(
      targetValue = if (state.enabled) 0f else maxTranslation,
      animationSpec = tween(300, easing = EaseOutCubic),
    )
  SideEffect {
    state.animatedAlpha = alpha
    state.animatedTranslationY = translationY
  }

  if (alpha == 0f) return

  Box(
    modifier
      .fillMaxSize()
      .graphicsLayer {
        this.alpha = alpha
        this.translationY = translationY
      }
      .then(if (state.enabled) Modifier else Modifier.pointerIgnore())
  ) {
    val isCustom = state.customKey != BottomBarState.NullKey
    if (isCustom) {
      state.customEntries[state.customKey]?.invoke()
    } else {
      state.pillEntries[state.pillKey]?.invoke()
      state.actionEntries[state.actionKey]?.invoke()
    }
  }
}
