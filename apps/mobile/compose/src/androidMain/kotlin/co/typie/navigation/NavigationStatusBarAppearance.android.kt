package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import co.typie.platform.activityContext

@Composable
internal actual fun ApplyPlatformNavigationStatusBarAppearance(
  useLightForeground: Boolean,
  defaultUseLightForeground: Boolean,
) {
  val activity = activityContext()
  val view = LocalView.current
  val insetsController =
    remember(activity, view) { WindowCompat.getInsetsController(activity.window, view) }

  SideEffect { insetsController.isAppearanceLightStatusBars = !useLightForeground }
  DisposableEffect(insetsController, defaultUseLightForeground) {
    onDispose { insetsController.isAppearanceLightStatusBars = !defaultUseLightForeground }
  }
}
