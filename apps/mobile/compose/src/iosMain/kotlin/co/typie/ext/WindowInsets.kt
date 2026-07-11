package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime as foundationIme
import androidx.compose.foundation.layout.navigationBars as foundationNavigationBars
import androidx.compose.foundation.layout.safeDrawing as foundationSafeDrawing
import androidx.compose.foundation.layout.statusBars as foundationStatusBars
import androidx.compose.runtime.Composable

actual val WindowInsets.Companion.statusBars: WindowInsets
  @Composable get() = foundationStatusBars

internal actual val WindowInsets.Companion.platformNavigationBars: WindowInsets
  @Composable get() = foundationNavigationBars

internal actual val WindowInsets.Companion.platformSafeDrawing: WindowInsets
  @Composable get() = foundationSafeDrawing

actual val WindowInsets.Companion.ime: WindowInsets
  @Composable get() = foundationIme
