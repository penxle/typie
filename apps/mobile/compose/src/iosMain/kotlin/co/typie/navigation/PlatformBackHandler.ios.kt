package co.typie.navigation

import androidx.compose.runtime.Composable
import kotlinx.coroutines.flow.Flow

@Composable
actual fun PlatformBackHandler(enabled: Boolean, onBack: () -> Unit) {
  // iOS: handled by edge swipe gesture in NavigationStack
}

@Composable
actual fun PlatformPredictiveBackHandler(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
) {
  // iOS: handled by edge swipe gesture in NavigationStack
}

@Composable actual fun systemBackGestureZoneWidth(): Float = 0f
