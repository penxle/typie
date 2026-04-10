@file:JvmName("WindowInsetsDesktopKt")

package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime as foundationIme
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp

// iPhone 16 Pro Max preview in desktop chrome:
// dynamic island 59pt, home indicator 34pt, frame bezel 12dp per side.
private val PreviewHorizontalSafeInset = 12.dp

actual val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get() = WindowInsets(top = 59.dp)

actual val WindowInsets.Companion.navigationBars: WindowInsets
  @Composable get() = WindowInsets(bottom = 34.dp)

actual val WindowInsets.Companion.safeDrawing: WindowInsets
  @Composable
  get() =
    WindowInsets(
      left = PreviewHorizontalSafeInset,
      top = 59.dp,
      right = PreviewHorizontalSafeInset,
      bottom = 34.dp,
    )

actual val WindowInsets.Companion.ime: WindowInsets
  @Composable get() = foundationIme
