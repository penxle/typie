package co.typie.ext

import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.unit.Velocity
import kotlin.math.abs
import kotlin.math.min
import kotlin.math.sign

private const val ELASTIC_OVERSCROLL_DAMPING = 0.4f
private const val ELASTIC_OVERSCROLL_SPRING_STIFFNESS = 400f

internal enum class ElasticAxis {
  Horizontal,
  Vertical,
}

internal class ElasticOverscrollState {
  var offset by mutableFloatStateOf(0f)

  fun consumePre(delta: Float): Float {
    if (offset == 0f || delta == 0f) return 0f

    val wouldReduce = (offset > 0f && delta > 0f) || (offset < 0f && delta < 0f)
    if (!wouldReduce) return 0f

    val maxConsumable = abs(offset) / ELASTIC_OVERSCROLL_DAMPING
    val consumed = min(abs(delta), maxConsumable) * sign(delta)
    offset -= consumed * ELASTIC_OVERSCROLL_DAMPING
    if (abs(offset) < 0.5f) {
      offset = 0f
    }

    return consumed
  }

  fun applyUnconsumed(delta: Float) {
    if (delta != 0f) {
      offset -= delta * ELASTIC_OVERSCROLL_DAMPING
    }
  }

  suspend fun release() {
    if (offset != 0f) {
      animate(
        initialValue = offset,
        targetValue = 0f,
        animationSpec = spring(stiffness = ELASTIC_OVERSCROLL_SPRING_STIFFNESS),
      ) { value, _ ->
        offset = value
      }
    }
  }
}

@Composable
internal fun rememberElasticOverscrollState(): ElasticOverscrollState = remember {
  ElasticOverscrollState()
}

internal fun shouldUseElasticOverscrollForDesktopDragScroll(
  enabled: Boolean,
  isLocked: Boolean,
  elasticOverscroll: Boolean,
): Boolean = enabled && !isLocked && elasticOverscroll

internal fun Orientation.toElasticAxis(): ElasticAxis =
  when (this) {
    Orientation.Horizontal -> ElasticAxis.Horizontal
    Orientation.Vertical -> ElasticAxis.Vertical
  }

internal fun ElasticAxis.extract(offset: Offset): Float =
  when (this) {
    ElasticAxis.Horizontal -> offset.x
    ElasticAxis.Vertical -> offset.y
  }

internal fun ElasticAxis.extract(velocity: Velocity): Float =
  when (this) {
    ElasticAxis.Horizontal -> velocity.x
    ElasticAxis.Vertical -> velocity.y
  }

internal fun ElasticAxis.pointerDelta(change: PointerInputChange): Float =
  when (this) {
    ElasticAxis.Horizontal -> change.position.x - change.previousPosition.x
    ElasticAxis.Vertical -> change.position.y - change.previousPosition.y
  }

internal fun ElasticAxis.offset(value: Float): Offset =
  when (this) {
    ElasticAxis.Horizontal -> Offset(value, 0f)
    ElasticAxis.Vertical -> Offset(0f, value)
  }

internal fun ElasticAxis.velocity(value: Float): Velocity =
  when (this) {
    ElasticAxis.Horizontal -> Velocity(value, 0f)
    ElasticAxis.Vertical -> Velocity(0f, value)
  }

internal fun Modifier.elasticOverscroll(
  axis: ElasticAxis,
  overscrollState: ElasticOverscrollState,
): Modifier = composed {
  val connection =
    remember(axis, overscrollState) {
      object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
          if (source != NestedScrollSource.UserInput) return Offset.Zero

          val delta = axis.extract(available)
          if (delta == 0f) return Offset.Zero
          return axis.offset(overscrollState.consumePre(delta))
        }

        override fun onPostScroll(
          consumed: Offset,
          available: Offset,
          source: NestedScrollSource,
        ): Offset = Offset.Zero

        override suspend fun onPreFling(available: Velocity): Velocity {
          overscrollState.release()
          return Velocity.Zero
        }

        override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
          overscrollState.release()
          return Velocity.Zero
        }
      }
    }

  clipToBounds().nestedScroll(connection).graphicsLayer {
    when (axis) {
      ElasticAxis.Horizontal -> translationX = overscrollState.offset
      ElasticAxis.Vertical -> translationY = overscrollState.offset
    }
  }
}
