package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.runtime.Composable

@Composable internal actual fun rememberTrustedImeInsets(): WindowInsets = WindowInsets.ime
