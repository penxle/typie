package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import co.typie.ui.component.topbar.TopBarForegroundStyle
import co.typie.ui.theme.ResolvedThemeMode

@Stable
internal class NavigationStatusBarAppearanceState {
  private val appearances = mutableStateMapOf<Any, TopBarForegroundStyle>()

  operator fun get(owner: Any): TopBarForegroundStyle? = appearances[owner]

  fun publish(owner: Any, style: TopBarForegroundStyle) {
    appearances[owner] = style
  }

  fun remove(owner: Any) {
    appearances.remove(owner)
  }
}

@Composable
internal fun ProvideNavigationStatusBarAppearanceOwner(
  owner: Any,
  state: NavigationStatusBarAppearanceState,
  content: @Composable () -> Unit,
) {
  val publisher =
    remember(owner, state) {
      NavigationStatusBarAppearancePublisher { style -> state.publish(owner, style) }
    }
  DisposableEffect(owner, state) { onDispose { state.remove(owner) } }
  CompositionLocalProvider(LocalNavigationStatusBarAppearancePublisher provides publisher) {
    content()
  }
}

@Composable
internal fun PublishNavigationStatusBarAppearance(style: TopBarForegroundStyle) {
  val publisher = LocalNavigationStatusBarAppearancePublisher.current ?: return
  SideEffect { publisher.publish(style) }
}

@Composable
internal fun ApplyActiveNavigationStatusBarAppearance(
  activeOwner: Any,
  state: NavigationStatusBarAppearanceState,
  themeMode: ResolvedThemeMode,
) {
  val defaultStyle = defaultNavigationTopBarLuminanceStyle(themeMode).foregroundStyle
  val style = state[activeOwner] ?: defaultStyle
  ApplyPlatformNavigationStatusBarAppearance(
    useLightForeground = style == TopBarForegroundStyle.Light,
    defaultUseLightForeground = defaultStyle == TopBarForegroundStyle.Light,
  )
}

private fun interface NavigationStatusBarAppearancePublisher {
  fun publish(style: TopBarForegroundStyle)
}

private val LocalNavigationStatusBarAppearancePublisher =
  staticCompositionLocalOf<NavigationStatusBarAppearancePublisher?> { null }

@Composable
internal expect fun ApplyPlatformNavigationStatusBarAppearance(
  useLightForeground: Boolean,
  defaultUseLightForeground: Boolean,
)
