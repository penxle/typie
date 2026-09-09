package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf

internal val LocalHardwareKeyboardConnected = staticCompositionLocalOf { false }

@Composable internal expect fun rememberHardwareKeyboardConnected(): Boolean
