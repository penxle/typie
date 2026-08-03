package co.typie.navigation

import androidx.compose.runtime.Composable
import kotlinx.coroutines.flow.Flow

@Composable
fun PlatformBackHandler(enabled: Boolean, onBack: () -> Unit) {
  PlatformBackHandlerImpl(
    enabled = enabled && LocalNavigationForegroundInteractive.current,
    onBack = onBack,
  )
}

@Composable internal expect fun PlatformBackHandlerImpl(enabled: Boolean, onBack: () -> Unit)

@Composable
fun PlatformPredictiveBackHandler(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
) {
  PlatformPredictiveBackHandlerImpl(
    enabled = enabled && LocalNavigationForegroundInteractive.current,
    onBack = onBack,
  )
}

@Composable
internal expect fun PlatformPredictiveBackHandlerImpl(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
)

@Composable expect fun systemBackGestureZoneWidth(): Float
