package co.typie

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.uikit.OnFocusBehavior
import androidx.compose.ui.window.ComposeUIViewController
import co.typie.navigation.LocalNavigationStatusBarAppearanceController
import co.typie.navigation.NavigationStatusBarAppearanceController
import co.typie.ui.utils.LocalNativeShortcutRegistry
import co.typie.ui.utils.NativeShortcutRegistry

fun MainViewController(
  shortcutRegistry: NativeShortcutRegistry,
  statusBarAppearanceController: NavigationStatusBarAppearanceController,
) =
  ComposeUIViewController(configure = { onFocusBehavior = OnFocusBehavior.DoNothing }) {
    CompositionLocalProvider(
      LocalNativeShortcutRegistry provides shortcutRegistry,
      LocalNavigationStatusBarAppearanceController provides statusBarAppearanceController,
    ) {
      App()
    }
  }
