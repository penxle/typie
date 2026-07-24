package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.test.Test
import kotlin.test.assertEquals

private const val SoftwareMagnifierRootTag = "software-magnifier-root"

@OptIn(ExperimentalTestApi::class)
class EditorSoftwareMagnifierDesktopTest {
  @Test
  fun lensSamplesSurfaceWithoutCapturingForeground() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(LocalAppColors provides LightColors) {
        val sourceLayer = rememberGraphicsLayer()
        Box(Modifier.size(320.dp).testTag(SoftwareMagnifierRootTag)) {
          Box(
            Modifier.fillMaxSize()
              .editorSoftwareMagnifierSource(sourceLayer = sourceLayer, active = true)
              .background(Color.Red)
          )
          Box(
            Modifier.fillMaxSize()
              .background(Color.Blue)
              .editorSoftwareMagnifierLens(
                sourceLayer = sourceLayer,
                placement =
                  EditorMagnifierPlacement(
                    sourceCenter = Offset(x = 160f, y = 200f),
                    magnifierCenter = Offset(x = 160f, y = 100f),
                    topLeft = Offset(x = 88f, y = 60f),
                  ),
              )
          )
        }
      }
    }
    waitForIdle()

    val pixels = onNodeWithTag(SoftwareMagnifierRootTag).captureToImage().toPixelMap()
    assertEquals(
      Color.Red,
      pixels[160, 100],
      "lens center should sample only the red editor surface",
    )
    assertEquals(Color.Blue, pixels[160, 200], "foreground outside the lens should remain blue")
  }
}
