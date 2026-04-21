package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime as foundationIme
import androidx.compose.foundation.layout.navigationBars as foundationNavigationBars
import androidx.compose.foundation.layout.safeDrawing as foundationSafeDrawing
import androidx.compose.foundation.layout.statusBars as foundationStatusBars
import androidx.compose.runtime.Composable

actual val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get() = foundationStatusBars

actual val WindowInsets.Companion.navigationBars: WindowInsets
  @Composable get() = foundationNavigationBars

actual val WindowInsets.Companion.safeDrawing: WindowInsets
  @Composable get() = foundationSafeDrawing

actual val WindowInsets.Companion.ime: WindowInsets
  @Composable get() = foundationIme
