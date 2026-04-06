package co.typie.ext

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier

expect val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get

expect val WindowInsets.Companion.navigationBars: WindowInsets
  @Composable get

expect val WindowInsets.Companion.safeDrawing: WindowInsets
  @Composable get

expect val WindowInsets.Companion.ime: WindowInsets
  @Composable get

@Composable
fun Modifier.statusBarsPadding(): Modifier =
  windowInsetsPadding(WindowInsets.statusBars)

@Composable
fun Modifier.navigationBarsPadding(): Modifier =
  windowInsetsPadding(WindowInsets.navigationBars)

@Composable
fun Modifier.safeDrawingPadding(): Modifier =
  windowInsetsPadding(WindowInsets.safeDrawing)

@Composable
fun Modifier.imePadding(): Modifier {
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val navBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
  val animatedBottom by animateDpAsState(
    targetValue = maxOf(imeBottom, navBottom),
    animationSpec = spring(
      dampingRatio = Spring.DampingRatioNoBouncy,
      stiffness = Spring.StiffnessHigh,
    ),
  )
  return padding(bottom = animatedBottom)
}
