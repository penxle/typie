package co.typie.ui.component.sheet

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class SheetOverlayDesktopTest {
  @Test
  fun entryCompletingDuringEntranceAnimationIsResolved() = runComposeUiTest {
    val sheet = Sheet()
    var result: String? = null

    setContent {
      SheetTestTheme {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          LaunchedEffect(Unit) {
            result = sheet.present {
              LaunchedEffect(Unit) { complete("done") }
              Box(Modifier.size(200.dp))
            }
          }

          SheetOverlay(sheet)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { result == "done" }

    assertEquals("done", result)
    assertEquals(0, sheet.entries.size)
  }

  @Composable
  private fun SheetTestTheme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeBlurStyle provides
        HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
      content = content,
    )
  }
}
