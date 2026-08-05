package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.uikit.LocalUIViewController

class NavigationStatusBarAppearanceController {
  private var lightForeground = false

  val useLightForeground: Boolean
    get() = lightForeground

  internal fun update(useLightForeground: Boolean) {
    lightForeground = useLightForeground
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
  val viewController = LocalUIViewController.current

  SideEffect {
    controller.update(useLightForeground)
    (viewController.parentViewController ?: viewController).setNeedsStatusBarAppearanceUpdate()
  }
  DisposableEffect(controller, viewController, defaultUseLightForeground) {
    onDispose {
      controller.update(defaultUseLightForeground)
      (viewController.parentViewController ?: viewController).setNeedsStatusBarAppearanceUpdate()
    }
  }
}
