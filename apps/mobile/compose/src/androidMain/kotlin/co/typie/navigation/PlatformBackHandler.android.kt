package co.typie.navigation

import androidx.activity.compose.BackHandler
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.systemGestures
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

@Composable
actual fun PlatformBackHandler(enabled: Boolean, onBack: () -> Unit) {
  BackHandler(enabled = enabled) { onBack() }
}

@Composable
actual fun PlatformPredictiveBackHandler(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
) {
  PredictiveBackHandler(enabled = enabled) { events -> onBack(events.map { it.progress }) }
}

@Composable
actual fun systemBackGestureZoneWidth(): Float =
  WindowInsets.systemGestures.getLeft(LocalDensity.current, LocalLayoutDirection.current).toFloat()
