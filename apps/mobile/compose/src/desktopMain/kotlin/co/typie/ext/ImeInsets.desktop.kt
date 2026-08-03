package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalDensity

@Composable
internal actual fun rememberTrustedImeBottomInset() =
  with(LocalDensity.current) { WindowInsets.ime.getBottom(this).toDp() }
