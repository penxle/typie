package co.typie.ext

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.foundation.clickable as foundationClickable

val LocalInteractionSource = compositionLocalOf<MutableInteractionSource?> { null }

@Composable
fun InteractionScope(content: @Composable () -> Unit) {
  val interactionSource = remember { MutableInteractionSource() }
  CompositionLocalProvider(LocalInteractionSource provides interactionSource) {
    content()
  }
}

expect fun Modifier.verticalScroll(state: ScrollState): Modifier
expect fun Modifier.horizontalScroll(state: ScrollState): Modifier
expect fun Modifier.overscroll(): Modifier

fun Modifier.clickable(onClick: () -> Unit): Modifier = composed {
  val interactionSource = LocalInteractionSource.current ?: remember { MutableInteractionSource() }
  foundationClickable(
    interactionSource = interactionSource,
    indication = null,
    onClick = onClick,
  )
}

fun Modifier.pressScale(targetScale: Float = 0.98f): Modifier = composed {
  val interactionSource = LocalInteractionSource.current ?: return@composed Modifier
  val scale = remember { Animatable(1f) }

  LaunchedEffect(interactionSource) {
    interactionSource.interactions.collect { interaction ->
      when (interaction) {
        is PressInteraction.Press -> scale.animateTo(targetScale, tween(100, easing = EaseOut))
        is PressInteraction.Release, is PressInteraction.Cancel -> scale.animateTo(
          1f,
          tween(100, easing = EaseOut)
        )
      }
    }
  }

  graphicsLayer {
    scaleX = scale.value
    scaleY = scale.value
  }
}
