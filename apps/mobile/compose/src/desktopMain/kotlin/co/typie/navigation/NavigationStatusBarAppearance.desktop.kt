package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf

internal class NavigationStatusBarAppearanceController {
  var useLightForeground: Boolean? by mutableStateOf(null)
    private set

  fun update(useLightForeground: Boolean) {
    this.useLightForeground = useLightForeground
  }
}

internal val LocalNavigationStatusBarAppearanceController =
  staticCompositionLocalOf<NavigationStatusBarAppearanceController?> { null }

@Composable
internal actual fun ApplyPlatformNavigationStatusBarAppearance(
  useLightForeground: Boolean,
  defaultUseLightForeground: Boolean,
) {
  val controller = LocalNavigationStatusBarAppearanceController.current ?: return

  SideEffect { controller.update(useLightForeground) }
  DisposableEffect(controller, defaultUseLightForeground) {
    onDispose { controller.update(defaultUseLightForeground) }
  }
}
