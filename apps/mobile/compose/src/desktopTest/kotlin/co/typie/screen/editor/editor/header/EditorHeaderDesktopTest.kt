package co.typie.screen.editor.editor.header

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorHeaderDesktopTest {
  @Test
  fun continuousHeaderAlignsTitleFieldToTheProvidedPageTrack() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.width(720.dp)) {
          EditorHeader(
            title = Title,
            subtitle = "",
            layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
            trackWidth = 640f,
            loading = false,
            topInset = 0.dp,
            onTitleChange = {},
            onSubtitleChange = {},
            onTitleFocused = {},
            onSubtitleFocused = {},
            onHeightChanged = {},
            onEnterDocument = {},
          )
        }
      }
    }
    waitForIdle()

    val titleWidth =
      onNode(hasText(Title) and hasSetTextAction(), useUnmergedTree = true)
        .fetchSemanticsNode()
        .boundsInRoot
        .width

    assertEquals(600f, titleWidth, absoluteTolerance = 0.01f)
  }

  private companion object {
    const val Title = "Document title"
  }
}
