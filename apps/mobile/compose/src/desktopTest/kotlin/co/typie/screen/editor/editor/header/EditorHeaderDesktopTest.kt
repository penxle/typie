package co.typie.screen.editor.editor.header

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performTextInputSelection
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

@OptIn(ExperimentalTestApi::class)
class EditorHeaderDesktopTest {
  @Test
  fun verticalArrowsExitOnlyWhenNativeMovementStaysOnVisualLine() = runComposeUiTest {
    val bodyEntries = mutableStateOf(0)

    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.width(160.dp)) {
          EditorHeader(
            title = LtrWrappedText,
            subtitle = SubtitleWrappedText,
            layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 120f),
            trackWidth = 160f,
            loading = false,
            topInset = 0.dp,
            onTitleChange = {},
            onSubtitleChange = {},
            onTitleFocused = {},
            onSubtitleFocused = {},
            onHeightChanged = {},
            onEnterDocument = { bodyEntries.value += 1 },
          )
        }
      }
    }
    waitForIdle()

    val title = onNode(hasText(LtrWrappedText) and hasSetTextAction(), useUnmergedTree = true)
    val subtitle =
      onNode(hasText(SubtitleWrappedText) and hasSetTextAction(), useUnmergedTree = true)

    fun selection(field: androidx.compose.ui.test.SemanticsNodeInteraction): TextRange =
      field.fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]

    fun press(field: androidx.compose.ui.test.SemanticsNodeInteraction, key: Key) {
      field.performKeyInput {
        keyDown(key)
        keyUp(key)
      }
      waitForIdle()
    }

    title.performClick()
    title.performTextInputSelection(TextRange(1))
    val titleBeforeDown = selection(title)
    press(title, Key.DirectionDown)
    title.assertIsFocused()
    assertNotEquals(titleBeforeDown, selection(title))

    title.performTextInputSelection(TextRange(LtrWrappedText.length - 1))
    press(title, Key.DirectionDown)
    subtitle.assertIsFocused()

    subtitle.performTextInputSelection(TextRange(SubtitleWrappedText.length - 1))
    val subtitleBeforeUp = selection(subtitle)
    press(subtitle, Key.DirectionUp)
    subtitle.assertIsFocused()
    assertNotEquals(subtitleBeforeUp, selection(subtitle))

    subtitle.performTextInputSelection(TextRange(1))
    press(subtitle, Key.DirectionUp)
    title.assertIsFocused()

    subtitle.performClick()
    subtitle.performTextInputSelection(TextRange(1))
    val subtitleBeforeDown = selection(subtitle)
    press(subtitle, Key.DirectionDown)
    subtitle.assertIsFocused()
    assertNotEquals(subtitleBeforeDown, selection(subtitle))

    subtitle.performTextInputSelection(TextRange(SubtitleWrappedText.length - 1))
    press(subtitle, Key.DirectionDown)
    assertEquals(1, bodyEntries.value)

    subtitle.performClick()
    subtitle.performTextInputSelection(TextRange(SubtitleWrappedText.length))
    subtitle.performKeyInput {
      keyDown(Key.ShiftLeft)
      keyDown(Key.DirectionDown)
      keyUp(Key.DirectionDown)
      keyUp(Key.ShiftLeft)
    }
    waitForIdle()
    subtitle.assertIsFocused()
    assertEquals(1, bodyEntries.value)

    subtitle.performTextInputSelection(TextRange(0, 3))
    press(subtitle, Key.DirectionDown)
    subtitle.assertIsFocused()
    assertEquals(1, bodyEntries.value)
  }

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

  @Test
  fun disabledHeaderExposesNoTextEditingAction() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorHeader(
          title = Title,
          subtitle = "",
          layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
          trackWidth = 640f,
          loading = false,
          enabled = false,
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
    waitForIdle()

    onAllNodes(hasText(Title) and hasSetTextAction(), useUnmergedTree = true).assertCountEquals(0)
  }

  private companion object {
    const val Title = "Document title"
    const val LtrWrappedText = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda"
    const val SubtitleWrappedText = "one two three four five six seven eight nine ten eleven twelve"
  }
}
