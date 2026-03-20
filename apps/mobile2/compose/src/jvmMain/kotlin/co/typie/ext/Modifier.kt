@file:JvmName("ModifierJvmKt")

package co.typie.ext

import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.Velocity
import kotlinx.coroutines.launch
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll

private const val DAMPING = 0.4f
private const val SPRING_STIFFNESS = 400f

actual fun Modifier.verticalScroll(state: ScrollState): Modifier = composed {
  var overscroll by remember { mutableFloatStateOf(0f) }
  val scope = rememberCoroutineScope()

  foundationVerticalScroll(state)
    .graphicsLayer { translationY = overscroll }
    .pointerInput(state) {
      detectDragGestures(
        onDragEnd = {
          scope.launch {
            animate(
              overscroll,
              0f,
              animationSpec = spring(stiffness = SPRING_STIFFNESS)
            ) { value, _ ->
              overscroll = value
            }
          }
        },
      ) { change, dragAmount ->
        change.consume()
        val delta = -dragAmount.y
        val consumed = state.dispatchRawDelta(delta)
        val unconsumed = delta - consumed
        if (unconsumed != 0f) {
          overscroll -= unconsumed * DAMPING
        }
      }
    }
}

actual fun Modifier.horizontalScroll(state: ScrollState): Modifier = composed {
  var overscroll by remember { mutableFloatStateOf(0f) }
  val scope = rememberCoroutineScope()

  foundationHorizontalScroll(state)
    .graphicsLayer { translationX = overscroll }
    .pointerInput(state) {
      detectDragGestures(
        onDragEnd = {
          scope.launch {
            animate(
              overscroll,
              0f,
              animationSpec = spring(stiffness = SPRING_STIFFNESS)
            ) { value, _ ->
              overscroll = value
            }
          }
        },
      ) { change, dragAmount ->
        change.consume()
        val delta = -dragAmount.x
        val consumed = state.dispatchRawDelta(delta)
        val unconsumed = delta - consumed
        if (unconsumed != 0f) {
          overscroll -= unconsumed * DAMPING
        }
      }
    }
}

actual fun Modifier.overscroll(): Modifier = composed {
  var offset by remember { mutableFloatStateOf(0f) }
  rememberCoroutineScope()

  val connection = remember {
    object : NestedScrollConnection {
      override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
        if (offset == 0f || source != NestedScrollSource.UserInput) return Offset.Zero

        val wouldReduce = (offset > 0f && available.y > 0f) || (offset < 0f && available.y < 0f)
        if (!wouldReduce) return Offset.Zero

        val old = offset
        offset -= available.y * DAMPING
        if ((old > 0f && offset < 0f) || (old < 0f && offset > 0f)) {
          offset = 0f
        }
        return available
      }

      override fun onPostScroll(
        consumed: Offset,
        available: Offset,
        source: NestedScrollSource
      ): Offset {
        if (available.y != 0f && source == NestedScrollSource.UserInput) {
          offset -= available.y * DAMPING
        }
        return Offset.Zero
      }

      override suspend fun onPreFling(available: Velocity): Velocity {
        if (offset != 0f) {
          animate(offset, 0f, animationSpec = spring(stiffness = SPRING_STIFFNESS)) { value, _ ->
            offset = value
          }
          return available
        }
        return Velocity.Zero
      }

      override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
        if (offset != 0f) {
          animate(offset, 0f, animationSpec = spring(stiffness = SPRING_STIFFNESS)) { value, _ ->
            offset = value
          }
        }
        return Velocity.Zero
      }
    }
  }

  nestedScroll(connection).graphicsLayer { translationY = offset }
}

