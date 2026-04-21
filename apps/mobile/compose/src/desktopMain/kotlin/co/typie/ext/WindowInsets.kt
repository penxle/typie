@file:JvmName("WindowInsetsDesktopKt")

package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime as foundationIme
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard

// iPhone 16 Pro Max preview in desktop chrome:
// dynamic island 59pt, home indicator 34pt, frame bezel 12dp per side.
// Content fills the whole window, so the bezel thickness is included in the
// status/navigation insets to match the visible screen area.
private val BezelThickness = 12.dp
private val DynamicIslandSafeTop = BezelThickness + 59.dp
private val HomeIndicatorSafeBottom = BezelThickness + 34.dp

actual val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get() = WindowInsets(top = DynamicIslandSafeTop)

actual val WindowInsets.Companion.navigationBars: WindowInsets
  @Composable get() = WindowInsets(bottom = HomeIndicatorSafeBottom)

actual val WindowInsets.Companion.safeDrawing: WindowInsets
  @Composable
  get() =
    WindowInsets(
      left = BezelThickness,
      top = DynamicIslandSafeTop,
      right = BezelThickness,
      bottom = HomeIndicatorSafeBottom,
    )

actual val WindowInsets.Companion.ime: WindowInsets
  @Composable get() = DesktopDebugKeyboard.asWindowInsets(foundationIme)
