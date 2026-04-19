package co.typie.ui.skeleton

import androidx.compose.animation.animateColor
import androidx.compose.animation.core.EaseInOut
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

data class SkeletonColors(val bone: Color, val highlight: Color)

class SkeletonState(val enabled: Boolean, val fraction: State<Float>, val boneColor: State<Color>) {
  companion object {
    val Disabled: SkeletonState =
      SkeletonState(
        enabled = false,
        fraction = mutableStateOf(0f),
        boneColor = mutableStateOf(Color.Transparent),
      )
  }
}

val LocalSkeleton = staticCompositionLocalOf { SkeletonState.Disabled }
val LocalSkeletonUnite = staticCompositionLocalOf<SkeletonUniteGroup?> { null }

object SkeletonDefaults {
  @Composable
  fun colors(
    bone: Color = AppTheme.colors.skeletonBase,
    highlight: Color = AppTheme.colors.skeletonShimmer,
  ): SkeletonColors = SkeletonColors(bone = bone, highlight = highlight)

  @Composable
  fun inverseColors(
    bone: Color = AppTheme.colors.skeletonBaseInverse,
    highlight: Color = AppTheme.colors.skeletonShimmerInverse,
  ): SkeletonColors = SkeletonColors(bone = bone, highlight = highlight)
}

@Composable
private fun rememberSkeletonState(enabled: Boolean, colors: SkeletonColors): SkeletonState {
  val transition = rememberInfiniteTransition()
  val fraction =
    animateFloatAsState(
      targetValue = if (enabled) 1f else 0f,
      animationSpec = tween(durationMillis = 200, easing = EaseInOut),
    )
  val boneColor =
    transition.animateColor(
      initialValue = colors.bone,
      targetValue = colors.highlight,
      animationSpec =
        infiniteRepeatable(
          animation = tween(durationMillis = 800, easing = EaseInOut),
          repeatMode = RepeatMode.Reverse,
        ),
    )
  return SkeletonState(enabled = enabled, fraction = fraction, boneColor = boneColor)
}

object Skeleton {
  @Composable
  operator fun invoke(
    enabled: Boolean,
    modifier: Modifier = Modifier,
    colors: SkeletonColors = SkeletonDefaults.colors(),
    content: @Composable () -> Unit,
  ) {
    val state = rememberSkeletonState(enabled, colors)
    Box(modifier = modifier.skeletonPointerIgnore(state)) {
      CompositionLocalProvider(LocalSkeleton provides state) { content() }
    }
  }

  @Composable
  fun Passive(
    enabled: Boolean,
    colors: SkeletonColors = SkeletonDefaults.colors(),
    content: @Composable () -> Unit,
  ) {
    val state = rememberSkeletonState(enabled, colors)
    CompositionLocalProvider(LocalSkeleton provides state) { content() }
  }

  @Composable
  fun Keep(content: @Composable () -> Unit) {
    CompositionLocalProvider(LocalSkeleton provides SkeletonState.Disabled) { content() }
  }

  @Composable
  fun Ignore(content: @Composable () -> Unit) {
    val state = LocalSkeleton.current
    Box(Modifier.graphicsLayer { alpha = 1f - state.fraction.value }) { content() }
  }

  @Composable
  fun Bone(
    modifier: Modifier = Modifier,
    shape: Shape = AppShapes.rounded(AppShapes.sm),
    content: @Composable () -> Unit = {},
  ) {
    Box(modifier = modifier.skeletonBone(shape)) {
      CompositionLocalProvider(LocalSkeleton provides SkeletonState.Disabled) { content() }
    }
  }

  @Composable
  fun Unite(shape: Shape = AppShapes.rounded(AppShapes.sm), content: @Composable () -> Unit) {
    val group = remember(shape) { SkeletonUniteGroup(shape) }
    CompositionLocalProvider(LocalSkeletonUnite provides group) { content() }
  }

  @Composable
  fun text(length: IntRange, lines: Int = 1): String {
    return remember(length, lines) { skeletonText(length, lines) }
  }

  @Composable
  fun <T> list(count: Int, item: SkeletonScope.() -> T): List<T> {
    return remember(count) { List(count) { SkeletonScope.item() } }
  }
}

object SkeletonScope {
  fun text(length: IntRange, lines: Int = 1): String = skeletonText(length, lines)
}

private const val FILLER = '\uAC00'

private fun skeletonText(length: IntRange, lines: Int = 1): String =
  (1..lines).joinToString("\n") { FILLER.toString().repeat(length.random()) }
