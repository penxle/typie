package co.typie.ext

import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable as foundationClickable
import androidx.compose.foundation.combinedClickable as foundationCombinedClickable
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import co.touchlab.kermit.Logger
import kotlinx.coroutines.isActive
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

@Composable
fun Modifier.verticalScroll(state: ScrollState, enabled: Boolean = true): Modifier {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  return this.foundationVerticalScroll(state, enabled = enabled && !isLocked)
}

@Composable
fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean = true): Modifier {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  return this.foundationHorizontalScroll(state, enabled = enabled && !isLocked)
}

@Composable
fun Modifier.clickable(onClick: suspend () -> Unit): Modifier =
  clickable(enabled = true, onClick = onClick)

@Composable
fun Modifier.clickable(
  enabled: Boolean = true,
  interactionSource: MutableInteractionSource =
    LocalInteractionSource.current ?: remember { MutableInteractionSource() },
  onClick: suspend () -> Unit,
): Modifier {
  val scope = rememberCoroutineScope()
  return this.focusProperties { canFocus = false }
    .foundationClickable(
      enabled = enabled,
      interactionSource = interactionSource,
      indication = null,
      onClick = {
        if (scope.isActive) {
          scope.launch { onClick() }
        } else {
          Logger.w { "clickable onClick dropped: coroutine scope is cancelled" }
        }
      },
    )
}

@Composable
fun Modifier.combinedClickable(
  enabled: Boolean = true,
  onClick: suspend () -> Unit,
  onLongClick: suspend () -> Unit,
): Modifier {
  val interactionSource = LocalInteractionSource.current ?: remember { MutableInteractionSource() }
  var handling by remember { mutableStateOf(false) }
  val scope = rememberCoroutineScope()
  return this.focusProperties { canFocus = false }
    .foundationCombinedClickable(
      enabled = enabled,
      interactionSource = interactionSource,
      indication = null,
      onClick = {
        if (!scope.isActive) {
          Logger.w { "combinedClickable onClick dropped: coroutine scope is cancelled" }
        } else if (!handling) {
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
        if (!scope.isActive) {
          Logger.w { "combinedClickable onLongClick dropped: coroutine scope is cancelled" }
        } else if (!handling) {
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

@Composable
fun Modifier.pressScale(targetScale: Float = 0.98f): Modifier {
  val interactionSource = LocalInteractionSource.current ?: return this
  val pressed by interactionSource.collectIsPressedAsState()
  val scale by
    animateFloatAsState(
      targetValue = if (pressed) targetScale else 1f,
      animationSpec = tween(100, easing = EaseOut),
    )

  return this.graphicsLayer {
    scaleX = scale
    scaleY = scale
  }
}
