package co.typie.screen.editor.editor.header

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performTextInputSelection
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorHeaderDesktopTest {
  @Test
  fun readingHeaderKeepsSelectionSemanticsWithoutTextMutation() = runComposeUiTest {
    var titleFocusCount = 0
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorHeader(
          title = Title,
          subtitle = "",
          loading = false,
          editing = false,
          topInset = 0.dp,
          onTitleChange = {},
          onSubtitleChange = {},
          onTitleFocused = { titleFocusCount += 1 },
          onSubtitleFocused = {},
          onHeightChanged = {},
          onEnterDocument = {},
        )
      }
    }
    waitForIdle()

    val field =
      onAllNodes(hasText(Title), useUnmergedTree = true).fetchSemanticsNodes().single {
        SemanticsProperties.TextSelectionRange in it.config
      }

    assertTrue(SemanticsActions.SetSelection in field.config)
    assertFalse(SemanticsActions.SetText in field.config)
    assertTrue(SemanticsActions.CustomActions in field.config)
    assertEquals(0, titleFocusCount)

    onNode(hasText(Title), useUnmergedTree = true).performTouchInput { click(center) }
    waitForIdle()

    assertEquals(1, titleFocusCount)
  }

  @Test
  fun readingTitleAccessibilityEditActionPromotesAndFocusesTheField() = runComposeUiTest {
    val editing = mutableStateOf(false)
    var promotionCount = 0
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorHeader(
          title = Title,
          subtitle = "",
          loading = false,
          editing = editing.value,
          topInset = 0.dp,
          onTitleChange = {},
          onSubtitleChange = {},
          onTitleFocused = {},
          onSubtitleFocused = {},
          onHeightChanged = {},
          onEnterDocument = {},
          onRequestEditing = {
            promotionCount += 1
            editing.value = true
            true
          },
        )
      }
    }
    waitForIdle()

    val readingTitle =
      onAllNodes(hasText(Title), useUnmergedTree = true).fetchSemanticsNodes().single {
        SemanticsProperties.TextSelectionRange in it.config
      }
    runOnIdle { assertTrue(readingTitle.config[SemanticsActions.CustomActions].single().action()) }
    waitForIdle()

    assertEquals(1, promotionCount)
    onNode(hasText(Title) and hasSetTextAction(), useUnmergedTree = true).assertIsFocused()
  }

  @Test
  fun readingTitleDoubleTapPromotesOnceAndPlacesCollapsedSelection() = runComposeUiTest {
    val editing = mutableStateOf(false)
    var promotionCount = 0
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.width(240.dp)) {
          EditorHeader(
            title = Title,
            subtitle = "",
            loading = false,
            editing = editing.value,
            topInset = 0.dp,
            onTitleChange = {},
            onSubtitleChange = {},
            onTitleFocused = {},
            onSubtitleFocused = {},
            onHeightChanged = {},
            onEnterDocument = {},
            onRequestEditing = {
              promotionCount += 1
              editing.value = true
              true
            },
          )
        }
      }
    }
    waitForIdle()

    val readingTitle = onNode(hasText(Title), useUnmergedTree = true)
    val readingTitleBounds = readingTitle.fetchSemanticsNode().boundsInRoot
    readingTitle.performTouchInput {
      val position = Offset(x = readingTitleBounds.width * 0.4f, y = readingTitleBounds.height / 2f)
      down(position)
      advanceEventTime(10)
      up()
      advanceEventTime(100)
      down(position)
      advanceEventTime(10)
      up()
    }
    waitForIdle()

    val editableTitle =
      onNode(hasText(Title) and hasSetTextAction(), useUnmergedTree = true).assertIsFocused()
    val selection =
      editableTitle.fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]
    assertEquals(1, promotionCount)
    assertTrue(selection.collapsed)
    assertTrue(selection.start in 1 until Title.length)
  }

  @Test
  fun readOnlyTitleDoubleTapKeepsNativeSelectionWithoutEditingActivation() = runComposeUiTest {
    var promotionCount = 0
    var hintCount = 0
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.width(240.dp)) {
          EditorHeader(
            title = Title,
            subtitle = "",
            loading = false,
            editing = false,
            editingActivationEnabled = false,
            topInset = 0.dp,
            onTitleChange = {},
            onSubtitleChange = {},
            onTitleFocused = {},
            onSubtitleFocused = {},
            onHeightChanged = {},
            onEnterDocument = {},
            onRequestEditing = {
              promotionCount += 1
              true
            },
            onReadingTapHint = { hintCount += 1 },
          )
        }
      }
    }
    waitForIdle()

    val title = onNode(hasText(Title), useUnmergedTree = true)
    val bounds = title.fetchSemanticsNode().boundsInRoot
    title.performTouchInput {
      val position = Offset(x = bounds.width * 0.4f, y = bounds.height / 2f)
      down(position)
      advanceEventTime(10)
      up()
      advanceEventTime(100)
      down(position)
      advanceEventTime(10)
      up()
    }
    waitForIdle()

    val selection =
      onAllNodes(hasText(Title), useUnmergedTree = true)
        .fetchSemanticsNodes()
        .single { SemanticsProperties.TextSelectionRange in it.config }
        .config[SemanticsProperties.TextSelectionRange]
    assertFalse(selection.collapsed)
    assertEquals(0, promotionCount)
    assertEquals(0, hintCount)
  }

  @Test
  fun readingTitleSingleTapPromotesWhenDoubleTapSettingIsDisabled() = runComposeUiTest {
    val editing = mutableStateOf(false)
    var promotionCount = 0
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorHeader(
          title = Title,
          subtitle = "",
          loading = false,
          editing = editing.value,
          doubleTapToEditEnabled = false,
          topInset = 0.dp,
          onTitleChange = {},
          onSubtitleChange = {},
          onTitleFocused = {},
          onSubtitleFocused = {},
          onHeightChanged = {},
          onEnterDocument = {},
          onRequestEditing = {
            promotionCount += 1
            editing.value = true
            true
          },
        )
      }
    }
    waitForIdle()

    onNode(hasText(Title), useUnmergedTree = true).performTouchInput { click(center) }
    waitForIdle()

    assertEquals(1, promotionCount)
    onNode(hasText(Title) and hasSetTextAction(), useUnmergedTree = true).assertIsFocused()
  }

  @Test
  fun editorReplacementClearsThePreviousHeaderReadingTap() = runComposeUiTest {
    val readingTapIdentity = mutableStateOf<Any>(Any())
    var promotionCount = 0
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorHeader(
          title = Title,
          subtitle = "",
          loading = false,
          editing = false,
          readingTapIdentity = readingTapIdentity.value,
          topInset = 0.dp,
          onTitleChange = {},
          onSubtitleChange = {},
          onTitleFocused = {},
          onSubtitleFocused = {},
          onHeightChanged = {},
          onEnterDocument = {},
          onRequestEditing = {
            promotionCount += 1
            true
          },
        )
      }
    }
    waitForIdle()

    onNode(hasText(Title), useUnmergedTree = true).performTouchInput { click(center) }
    runOnIdle { readingTapIdentity.value = Any() }
    waitForIdle()
    onNode(hasText(Title), useUnmergedTree = true).performTouchInput { click(center) }
    waitForIdle()

    assertEquals(0, promotionCount)
  }

  @Test
  fun readingCleanupCollapsesExistingHeaderSelection() = runComposeUiTest {
    val editing = mutableStateOf(true)
    val cleanupRequest = mutableStateOf(0)
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorHeader(
          title = Title,
          subtitle = SubtitleWrappedText,
          loading = false,
          editing = editing.value,
          readingModeCleanupRequest = cleanupRequest.value,
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

    onNode(hasText(Title) and hasSetTextAction(), useUnmergedTree = true)
      .performTextInputSelection(TextRange(1, 5))
    editing.value = false
    cleanupRequest.value += 1
    waitForIdle()

    val field =
      onAllNodes(hasText(Title), useUnmergedTree = true).fetchSemanticsNodes().single {
        SemanticsProperties.TextSelectionRange in it.config
      }
    assertTrue(field.config[SemanticsProperties.TextSelectionRange].collapsed)
  }

  @Test
  fun verticalArrowsExitOnlyWhenNativeMovementStaysOnVisualLine() = runComposeUiTest {
    val bodyEntries = mutableStateOf(0)
    var titleFocusCount = 0
    var subtitleFocusCount = 0

    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.width(160.dp)) {
          EditorHeader(
            title = LtrWrappedText,
            subtitle = SubtitleWrappedText,
            loading = false,
            topInset = 0.dp,
            onTitleChange = {},
            onSubtitleChange = {},
            onTitleFocused = { titleFocusCount += 1 },
            onSubtitleFocused = { subtitleFocusCount += 1 },
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
    waitForIdle()
    assertEquals(1, titleFocusCount)
    title.performTextInputSelection(TextRange(1))
    val titleBeforeDown = selection(title)
    press(title, Key.DirectionDown)
    title.assertIsFocused()
    assertNotEquals(titleBeforeDown, selection(title))

    title.performTextInputSelection(TextRange(LtrWrappedText.length - 1))
    press(title, Key.DirectionDown)
    subtitle.assertIsFocused()
    assertEquals(1, subtitleFocusCount)

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
          EditorHeaderFrame(
            geometry =
              checkNotNull(
                resolveEditorHeaderGeometry(
                  layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
                  viewportWidth = 720f,
                  bodyTrackWidth = 640f,
                  displayZoom = 1f,
                )
              )
          ) {
            EditorHeader(
              title = Title,
              subtitle = "",
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
