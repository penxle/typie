package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.dp

expect val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get

internal expect val WindowInsets.Companion.platformNavigationBars: WindowInsets
  @Composable get

internal expect val WindowInsets.Companion.platformSafeDrawing: WindowInsets
  @Composable get

expect val WindowInsets.Companion.ime: WindowInsets
  @Composable get

private val MinBottomSafeArea = WindowInsets(bottom = 16.dp)

val WindowInsets.Companion.navigationBars: WindowInsets
  @Composable get() = WindowInsets.platformNavigationBars.union(MinBottomSafeArea)

val WindowInsets.Companion.safeDrawing: WindowInsets
  @Composable get() = WindowInsets.platformSafeDrawing.union(MinBottomSafeArea)

val WindowInsets.Companion.safeDrawingHorizontal: WindowInsets
  @Composable
  get() {
    val direction = LocalLayoutDirection.current
    val values = WindowInsets.safeDrawing.asPaddingValues()
    return WindowInsets(
      left = values.calculateLeftPadding(direction),
      right = values.calculateRightPadding(direction),
    )
  }

@Composable
fun Modifier.statusBarsPadding(): Modifier = windowInsetsPadding(WindowInsets.statusBars)

@Composable
fun Modifier.navigationBarsPadding(): Modifier = windowInsetsPadding(WindowInsets.navigationBars)

@Composable fun Modifier.imePadding(): Modifier = windowInsetsPadding(WindowInsets.ime)

@Composable
fun Modifier.navigationBarsOrImePadding(): Modifier {
  val navigationBarsBottom = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  return windowInsetsPadding(WindowInsets(bottom = maxOf(navigationBarsBottom, imeBottom)))
}

@Composable
fun Modifier.safeDrawingHorizontalPadding(): Modifier {
  val direction = LocalLayoutDirection.current
  val values = WindowInsets.safeDrawing.asPaddingValues()
  return windowInsetsPadding(
    WindowInsets(
      left = values.calculateLeftPadding(direction),
      right = values.calculateRightPadding(direction),
    )
  )
}

@Composable
fun Modifier.safeDrawingStartPadding(): Modifier {
  val direction = LocalLayoutDirection.current
  val values = WindowInsets.safeDrawing.asPaddingValues()
  return windowInsetsPadding(WindowInsets(left = values.calculateLeftPadding(direction)))
}
