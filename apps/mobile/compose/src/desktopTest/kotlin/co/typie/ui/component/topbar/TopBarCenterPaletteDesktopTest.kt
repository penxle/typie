package co.typie.ui.component.topbar

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.navigation.NavigationScaffold
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.navigation.PublishNavigationTopBarBackdropStyle
import co.typie.route.Route
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.DarkColors
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class TopBarCenterPaletteDesktopTest {
  @Test
  fun navigationStackUsesTheAmbientDarkThemeForBrightContent() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val topBarState = TopBarState()

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides DarkColors,
        LocalThemeMode provides ResolvedThemeMode.Dark,
      ) {
        NavigationScaffold(navigator = navigator, topBarState = topBarState) {
          NavigationStack(navigator = navigator, topBarState = topBarState) {
            Box(Modifier.fillMaxSize().background(Color.White))
            PublishNavigationTopBarBackdropStyle(Color.White)
            ProvideTopBar(center = { PaletteSample("dark-theme-center") })
          }
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) {
      topBarState.adaptiveForegroundStyle == TopBarForegroundStyle.Light
    }
  }

  @Test
  fun lightThemeDarkNavigationSurfaceInvertsTheScaffoldCenterSlot() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val sourceColor = mutableStateOf(Color.Black)
    val topBarState = TopBarState()

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        NavigationScaffold(navigator = navigator, topBarState = topBarState) {
          NavigationStack(navigator = navigator, topBarState = topBarState) {
            Box(Modifier.fillMaxSize().background(sourceColor.value))
            PublishNavigationTopBarBackdropStyle(sourceColor.value)
            ProvideTopBar(
              leading = { PaletteSample("integrated-leading") },
              center = { PaletteSample("integrated-center") },
              trailing = { PaletteSample("integrated-trailing") },
            )
          }
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { onNodeWithTag("integrated-center").centerLuminance() > 0.8f }
    assertTrue(onNodeWithTag("integrated-leading").centerLuminance() < 0.2f)
    assertTrue(onNodeWithTag("integrated-trailing").centerLuminance() < 0.2f)

    sourceColor.value = Color.White
    waitUntil(timeoutMillis = 5_000) {
      topBarState.adaptiveForegroundStyle == TopBarForegroundStyle.Dark
    }
    waitUntil(timeoutMillis = 5_000) { onNodeWithTag("integrated-center").centerLuminance() < 0.2f }
    assertTrue(onNodeWithTag("integrated-leading").centerLuminance() < 0.2f)
    assertTrue(onNodeWithTag("integrated-trailing").centerLuminance() < 0.2f)
  }

  @Test
  fun themeSurfaceCenterKeepsTheAmbientPalette() = runComposeUiTest {
    setContent {
      val state = remember {
        TopBarState().apply {
          adaptiveForegroundStyle = TopBarForegroundStyle.Light
          setLeading("leading", { PaletteSample("leading") })
          setCenter(
            "center",
            { PaletteSample("center") },
            appearance = TopBarCenterAppearance.ThemeSurface,
          )
          setTrailing("trailing", { PaletteSample("trailing") })
        }
      }

      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(width = 320.dp, height = 100.dp)) { TopBar(state) }
      }
    }
    waitForIdle()

    val leadingLuminance = onNodeWithTag("leading").centerLuminance()
    val centerLuminance = onNodeWithTag("center").centerLuminance()
    val trailingLuminance = onNodeWithTag("trailing").centerLuminance()

    assertTrue(centerLuminance < 0.2f, "center luminance=$centerLuminance")
    assertTrue(leadingLuminance < 0.2f, "leading luminance=$leadingLuminance")
    assertTrue(trailingLuminance < 0.2f, "trailing luminance=$trailingLuminance")
  }
}

@androidx.compose.runtime.Composable
private fun PaletteSample(tag: String) {
  Box(Modifier.size(16.dp).testTag(tag).background(AppTheme.colors.textDefault))
}

private fun androidx.compose.ui.test.SemanticsNodeInteraction.centerLuminance(): Float {
  val pixels = captureToImage().toPixelMap()
  return pixels[pixels.width / 2, pixels.height / 2].luminance()
}
