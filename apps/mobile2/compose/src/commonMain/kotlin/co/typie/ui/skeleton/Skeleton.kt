package co.typie.ui.skeleton

import androidx.compose.animation.core.EaseInOut
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.unit.dp
import co.typie.ext.pointerIgnore
import co.typie.ui.theme.AppTheme

data class SkeletonColors(
  val bone: Color,
  val highlight: Color,
)

class SkeletonState(
  val enabled: Boolean,
  val color: State<Color>,
  val colors: SkeletonColors,
) {
  companion object {
    val Disabled = SkeletonState(
      enabled = false,
      color = mutableStateOf(Color.Transparent),
      colors = SkeletonColors(Color.Transparent, Color.Transparent),
    )
  }
}

val LocalSkeleton = staticCompositionLocalOf { SkeletonState.Disabled }
val LocalSkeletonUnite = staticCompositionLocalOf<SkeletonUniteGroup?> { null }

class SkeletonUniteGroup {
  private val members = mutableListOf<LayoutCoordinates>()

  fun clear() {
    members.clear()
  }

  fun register(coordinates: LayoutCoordinates) {
    members.add(coordinates)
  }

  fun getUnionBounds(relativeTo: LayoutCoordinates): Rect {
    var union: Rect? = null
    for (member in members) {
      if (!member.isAttached) continue
      val bounds = relativeTo.localBoundingBoxOf(member, clipBounds = false)
      union = union?.let {
        Rect(
          left = minOf(it.left, bounds.left),
          top = minOf(it.top, bounds.top),
          right = maxOf(it.right, bounds.right),
          bottom = maxOf(it.bottom, bounds.bottom),
        )
      } ?: bounds
    }
    return union ?: Rect.Zero
  }
}

object SkeletonDefaults {
  @Composable
  fun colors(
    bone: Color = AppTheme.colors.skeletonBone,
    highlight: Color = AppTheme.colors.skeletonHighlight,
  ): SkeletonColors = SkeletonColors(bone = bone, highlight = highlight)
}

object Skeleton {
  @Composable
  operator fun invoke(
    enabled: Boolean,
    modifier: Modifier = Modifier,
    colors: SkeletonColors = SkeletonDefaults.colors(),
    content: @Composable () -> Unit,
  ) {
    val fadeAlpha by animateFloatAsState(
      targetValue = if (enabled) 1f else 0f,
      animationSpec = tween(200),
    )
    val effectiveAlpha = if (enabled) 1f else fadeAlpha
    val active = enabled || fadeAlpha > 0f

    Box(modifier) {
      // 실제 콘텐츠 (항상 렌더링)
      CompositionLocalProvider(LocalSkeleton provides SkeletonState.Disabled) {
        content()
      }

      // Bone 오버레이 (fade out)
      if (active) {
        val infiniteTransition = rememberInfiniteTransition()
        val pulseAlpha by infiniteTransition.animateFloat(
          initialValue = 0f,
          targetValue = 1f,
          animationSpec = infiniteRepeatable<Float>(
            animation = tween<Float>(800, easing = EaseInOut),
            repeatMode = RepeatMode.Reverse,
          ),
        )

        val animatedColor = remember { mutableStateOf(colors.bone) }
        animatedColor.value = lerp(colors.bone, colors.highlight, pulseAlpha)

        val state = remember(colors) {
          SkeletonState(enabled = true, color = animatedColor, colors = colors)
        }

        CompositionLocalProvider(LocalSkeleton provides state) {
          Box(Modifier.alpha(effectiveAlpha)) {
            content()
          }
        }
      }

      // 터치 차단 레이어 (최상단)
      if (enabled) {
        Box(Modifier.matchParentSize().pointerIgnore())
      }
    }
  }

  @Composable
  fun Ignore(content: @Composable () -> Unit) {
    val skeleton = LocalSkeleton.current
    if (skeleton.enabled) {
      Box(Modifier.alpha(0f)) { content() }
    } else {
      content()
    }
  }

  @Composable
  fun Bone(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(4.dp),
    content: @Composable () -> Unit,
  ) {
    val skeleton = LocalSkeleton.current
    if (skeleton.enabled) {
      SkeletonBone(modifier = modifier, shape = shape)
    } else {
      content()
    }
  }

  @Composable
  fun Keep(content: @Composable () -> Unit) {
    val skeleton = LocalSkeleton.current
    if (skeleton.enabled) {
      CompositionLocalProvider(LocalSkeleton provides SkeletonState.Disabled) {
        content()
      }
    } else {
      content()
    }
  }

  @Composable
  fun Replace(
    replacement: @Composable () -> Unit,
    content: @Composable () -> Unit,
  ) {
    val skeleton = LocalSkeleton.current
    if (skeleton.enabled) {
      replacement()
    } else {
      content()
    }
  }

  @Composable
  fun Unite(
    shape: Shape = RoundedCornerShape(4.dp),
    content: @Composable () -> Unit,
  ) {
    val skeleton = LocalSkeleton.current
    if (!skeleton.enabled) {
      content()
      return
    }

    val group = remember { SkeletonUniteGroup() }
    SideEffect { group.clear() }

    CompositionLocalProvider(LocalSkeletonUnite provides group) {
      content()
    }
  }

  @Composable
  fun text(length: IntRange, lines: Int = 1): String {
    return remember(length, lines) { skeletonText(length, lines) }
  }

  @Composable
  fun <T> list(count: Int, item: SkeletonScope.() -> T): List<T> {
    return remember(count) {
      List(count) { SkeletonScope.item() }
    }
  }
}

object SkeletonScope {
  fun text(length: IntRange, lines: Int = 1): String = skeletonText(length, lines)
}

private const val FILLER = '\uAC00'

private fun skeletonText(length: IntRange, lines: Int = 1): String =
  (1..lines).joinToString("\n") { FILLER.toString().repeat(length.random()) }
