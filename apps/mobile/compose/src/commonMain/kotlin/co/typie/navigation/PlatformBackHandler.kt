package co.typie.navigation

import androidx.compose.runtime.Composable
import kotlinx.coroutines.flow.Flow

@Composable expect fun PlatformBackHandler(enabled: Boolean, onBack: () -> Unit)

@Composable
expect fun PlatformPredictiveBackHandler(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
)

@Composable expect fun systemBackGestureZoneWidth(): Float
