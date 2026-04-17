package co.typie.ext

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable as foundationClickable
import androidx.compose.foundation.combinedClickable as foundationCombinedClickable
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll
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
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

val LocalInteractionSource = compositionLocalOf<MutableInteractionSource?> { null }

inline fun <T> Modifier.thenIfNotNull(value: T?, builder: Modifier.(T) -> Modifier): Modifier =
  if (value != null) builder(value) else this

inline fun Modifier.thenIf(condition: Boolean, builder: Modifier.() -> Modifier): Modifier =
  if (condition) builder() else this

@Composable
fun InteractionScope(content: @Composable () -> Unit) {
  val interactionSource = remember { MutableInteractionSource() }
  CompositionLocalProvider(LocalInteractionSource provides interactionSource) { content() }
}

fun Modifier.verticalScroll(
  state: ScrollState,
  enabled: Boolean = true,
  padding: PaddingValues = AppTheme.spacings.scrollBottomPadding,
): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  foundationVerticalScroll(state, enabled = enabled && !isLocked) then this.padding(padding)
}

fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean = true): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  foundationHorizontalScroll(state, enabled = enabled && !isLocked)
}

fun Modifier.clickable(onClick: suspend () -> Unit): Modifier =
  clickable(enabled = true, onClick = onClick)

fun Modifier.clickable(enabled: Boolean = true, onClick: suspend () -> Unit): Modifier = composed {
  val scope = rememberCoroutineScope()
  val interactionSource = LocalInteractionSource.current ?: remember { MutableInteractionSource() }
  focusProperties { canFocus = false }
    .foundationClickable(
      enabled = enabled,
      interactionSource = interactionSource,
      indication = null,
      onClick = { scope.launch { onClick() } },
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

fun Modifier.pointerIgnore(): Modifier =
  pointerInput(Unit) {
    awaitPointerEventScope {
      while (true) {
        awaitPointerEvent().changes.forEach { it.consume() }
      }
    }
  }

fun Modifier.pressScale(targetScale: Float = 0.98f): Modifier = composed {
  val interactionSource = LocalInteractionSource.current ?: return@composed Modifier
  val scale = remember { Animatable(1f) }

  LaunchedEffect(interactionSource) {
    interactionSource.interactions.collect { interaction ->
      when (interaction) {
        is PressInteraction.Press -> scale.animateTo(targetScale, tween(100, easing = EaseOut))
        is PressInteraction.Release,
        is PressInteraction.Cancel -> scale.animateTo(1f, tween(100, easing = EaseOut))
      }
    }
  }

  graphicsLayer {
    scaleX = scale.value
    scaleY = scale.value
  }
}
