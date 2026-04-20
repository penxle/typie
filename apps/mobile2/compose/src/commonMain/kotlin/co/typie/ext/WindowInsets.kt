package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLayoutDirection

expect val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get

expect val WindowInsets.Companion.navigationBars: WindowInsets
  @Composable get

expect val WindowInsets.Companion.safeDrawing: WindowInsets
  @Composable get

expect val WindowInsets.Companion.ime: WindowInsets
  @Composable get

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
