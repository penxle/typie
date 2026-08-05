package co.typie.navigation

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NavigationTopBarBackdropTest {
  @Test
  fun visibleMainRouteUsesPublishedBackgroundAndPresence() {
    val style =
      resolveNavigationTopBarBackdropStyle(
        behindBackground = null,
        behindPresence = 0f,
        mainBackground = Color.Red,
        mainPresence = 1f,
        mainWeight = 1f,
        fallbackBackground = Color.Black,
      )

    assertEquals(Color.Red, style.background)
    assertEquals(1f, style.presence)
  }

  @Test
  fun routeWithoutTopBarContributesZeroPresence() {
    val style =
      resolveNavigationTopBarBackdropStyle(
        behindBackground = Color.Red,
        behindPresence = 1f,
        mainBackground = Color.Blue,
        mainPresence = 0f,
        mainWeight = 0.25f,
        fallbackBackground = Color.Black,
      )

    assertEquals(0.75f, style.presence)
  }

  @Test
  fun transitionInterpolatesBackgroundAndPresenceContinuously() {
    val midpoint =
      resolveNavigationTopBarBackdropStyle(
        behindBackground = Color.Red,
        behindPresence = 0f,
        mainBackground = Color.Blue,
        mainPresence = 1f,
        mainWeight = 0.5f,
        fallbackBackground = Color.Black,
      )

    assertEquals(lerp(Color.Red, Color.Blue, 0.5f), midpoint.background)
    assertEquals(0.5f, midpoint.presence)
  }

  @Test
  fun completedPopMatchesDestinationIdleStyle() {
    val beforeRoleSwitch =
      resolveNavigationTopBarBackdropStyle(
        behindBackground = Color.Red,
        behindPresence = 1f,
        mainBackground = Color.Blue,
        mainPresence = 0f,
        mainWeight = 0f,
        fallbackBackground = Color.Black,
      )
    val afterRoleSwitch =
      resolveNavigationTopBarBackdropStyle(
        behindBackground = null,
        behindPresence = 0f,
        mainBackground = Color.Red,
        mainPresence = 1f,
        mainWeight = 1f,
        fallbackBackground = Color.Black,
      )

    assertEquals(afterRoleSwitch, beforeRoleSwitch)
  }

  @Test
  fun missingPublicationUsesFallbackBackground() {
    val style =
      resolveNavigationTopBarBackdropStyle(
        behindBackground = null,
        behindPresence = 1f,
        mainBackground = null,
        mainPresence = 1f,
        mainWeight = 0.4f,
        fallbackBackground = Color.Green,
      )

    assertEquals(Color.Green, style.background)
  }

  @Test
  fun interactiveTransitionKeepsTheSourceAppearance() {
    val sourceLuminance = NavigationTopBarContentLuminance.Dark
    val destinationLuminance = NavigationTopBarContentLuminance.Bright

    val appearance =
      resolveNavigationTopBarTransitionAppearance(
        themeMode = ResolvedThemeMode.Light,
        committed = false,
        sourceBlurEnabled = false,
        sourceContentLuminance = sourceLuminance,
        destinationBlurEnabled = true,
        destinationContentLuminance = destinationLuminance,
      )

    assertFalse(appearance.blurEnabled)
    assertEquals(sourceLuminance, appearance.contentLuminance)
  }

  @Test
  fun committedTransitionUsesDestinationBlurAndVerifiedFade() {
    val sourceLuminance = NavigationTopBarContentLuminance.Dark
    val destinationLuminance = NavigationTopBarContentLuminance.Bright

    val appearance =
      resolveNavigationTopBarTransitionAppearance(
        themeMode = ResolvedThemeMode.Light,
        committed = true,
        sourceBlurEnabled = false,
        sourceContentLuminance = sourceLuminance,
        destinationBlurEnabled = true,
        destinationContentLuminance = destinationLuminance,
      )

    assertTrue(appearance.blurEnabled)
    assertEquals(destinationLuminance, appearance.contentLuminance)
  }

  @Test
  fun committedTransitionUsesTheThemeSafeFallbackForAnUnmeasuredDestination() {
    val lightAppearance =
      resolveNavigationTopBarTransitionAppearance(
        themeMode = ResolvedThemeMode.Light,
        committed = true,
        sourceBlurEnabled = false,
        sourceContentLuminance = NavigationTopBarContentLuminance.Dark,
        destinationBlurEnabled = true,
        destinationContentLuminance = null,
      )
    val darkAppearance =
      resolveNavigationTopBarTransitionAppearance(
        themeMode = ResolvedThemeMode.Dark,
        committed = true,
        sourceBlurEnabled = false,
        sourceContentLuminance = NavigationTopBarContentLuminance.Bright,
        destinationBlurEnabled = true,
        destinationContentLuminance = null,
      )

    assertTrue(lightAppearance.blurEnabled)
    assertEquals(NavigationTopBarContentLuminance.Bright, lightAppearance.contentLuminance)
    assertTrue(darkAppearance.blurEnabled)
    assertEquals(NavigationTopBarContentLuminance.Dark, darkAppearance.contentLuminance)
  }
}
