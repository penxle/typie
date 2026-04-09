package co.typie.ext

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.input.pointer.pointerInput
import kotlinx.coroutines.launch
import androidx.compose.foundation.clickable as foundationClickable
import androidx.compose.foundation.combinedClickable as foundationCombinedClickable

val LocalInteractionSource = compositionLocalOf<MutableInteractionSource?> { null }

@Composable
fun InteractionScope(content: @Composable () -> Unit) {
  val interactionSource = remember { MutableInteractionSource() }
  CompositionLocalProvider(LocalInteractionSource provides interactionSource) {
    content()
  }
}

expect fun Modifier.verticalScroll(state: ScrollState, enabled: Boolean = true): Modifier
expect fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean = true): Modifier
expect fun Modifier.desktopScrollBehavior(
  state: ScrollableState,
  orientation: Orientation,
  enabled: Boolean = true,
): Modifier
internal expect fun Modifier.desktopDragScroll(
  state: ScrollableState,
  orientation: Orientation,
  enabled: Boolean = true,
): Modifier

fun Modifier.clickable(onClick: suspend () -> Unit): Modifier = clickable(
  enabled = true,
  onClick = onClick,
)

fun Modifier.clickable(
  enabled: Boolean = true,
  onClick: suspend () -> Unit,
): Modifier = composed {
  val interactionSource = LocalInteractionSource.current ?: remember { MutableInteractionSource() }
  var handling by remember { mutableStateOf(false) }
  val scope = rememberCoroutineScope()
  focusProperties { canFocus = false }
    .foundationClickable(
      enabled = enabled,
      interactionSource = interactionSource,
      indication = null,
      onClick = {
        if (!handling) {
          handling = true
          scope.launch {
            try {
              onClick()
            } finally {
              handling = false
            }
          }
        }
      },
    )
}

fun Modifier.combinedClickable(
  enabled: Boolean = true,
  onClick: suspend () -> Unit,
  onLongClick: suspend () -> Unit,
): Modifier = composed {
  val interactionSource = LocalInteractionSource.current ?: remember { MutableInteractionSource() }
  var handling by remember { mutableStateOf(false) }
  val scope = rememberCoroutineScope()
  focusProperties { canFocus = false }
    .foundationCombinedClickable(
      enabled = enabled,
      interactionSource = interactionSource,
      indication = null,
      onClick = {
        if (!handling) {
          handling = true
          scope.launch {
            try {
              onClick()
            } finally {
              handling = false
            }
          }
        }
      },
      onLongClick = {
        if (!handling) {
          handling = true
          scope.launch {
            try {
              onLongClick()
            } finally {
              handling = false
            }
          }
        }
      },
    )
}

fun Modifier.pointerIgnore(): Modifier = pointerInput(Unit) {
  awaitPointerEventScope { while (true) { awaitPointerEvent().changes.forEach { it.consume() } } }
}

internal fun Modifier.touchShield(): Modifier = pointerInput(Unit) {}

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
