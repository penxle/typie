package co.typie.navigation

import androidx.compose.runtime.Composable
import kotlinx.coroutines.flow.Flow

@Composable
internal actual fun PlatformBackHandlerImpl(enabled: Boolean, onBack: () -> Unit) {
  // iOS: handled by edge swipe gesture in NavigationStack
}

@Composable
internal actual fun PlatformPredictiveBackHandlerImpl(
  enabled: Boolean,
  onBack: suspend (progress: Flow<Float>) -> Unit,
) {
  // iOS: handled by edge swipe gesture in NavigationStack
}

@Composable actual fun systemBackGestureZoneWidth(): Float = 0f
