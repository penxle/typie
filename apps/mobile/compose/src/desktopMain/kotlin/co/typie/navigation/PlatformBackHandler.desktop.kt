package co.typie.navigation

import androidx.compose.runtime.Composable
import kotlinx.coroutines.flow.Flow

@Composable
internal actual fun PlatformBackHandlerImpl(enabled: Boolean, onBack: () -> Unit) {
  // Desktop: no system back button
}

@Composable
internal actual fun PlatformPredictiveBackHandlerImpl(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
) {
  // Desktop: no system back gesture
}

@Composable actual fun systemBackGestureZoneWidth(): Float = 0f
