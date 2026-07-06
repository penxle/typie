package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.runComposeUiTest
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorContextMenuOverlayDesktopTest {
  @Test
  fun selectingExpansionShowsLegacyExpansionMenu() = runComposeUiTest {
    setMenuContent()

    waitForIdle()
    assertEquals(0, onAllNodesWithText("단어").fetchSemanticsNodes().size)

    onNodeWithText("선택 확장").performClick()
    waitForIdle()

    assertEquals(0, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("단어").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("문장").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("문단").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("전체").fetchSemanticsNodes().size)
  }

  @Test
  fun selectingExpansionShowsOnlyAvailableExpansionActions() = runComposeUiTest {
    setMenuContent(
      availableExpansionUnits = setOf(SelectionExpansionUnit.Word, SelectionExpansionUnit.Paragraph)
    )

    waitForIdle()
    onNodeWithText("선택 확장").performClick()
    waitForIdle()

    assertEquals(1, onAllNodesWithText("단어").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("문장").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("문단").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("전체").fetchSemanticsNodes().size)
  }

  @Test
  fun primaryMenuHidesSelectionExpansionWhenNoUnitIsAvailable() = runComposeUiTest {
    setMenuContent(availableExpansionUnits = emptySet())

    waitForIdle()

    assertEquals(0, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
  }

  @Test
  fun collapsedMenuHidesRangeOnlyActionsButKeepsSelectionExpansion() = runComposeUiTest {
    setMenuContent(showCopyCutActions = false)

    waitForIdle()

    assertEquals(0, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
  }

  @Test
  fun primaryMenuItemInvokesActionAndDismisses() = runComposeUiTest {
    var copyCount = 0
    var dismissCount = 0
    setMenuContent(onCopy = { copyCount++ }, onDismiss = { dismissCount++ })

    waitForIdle()
    onNodeWithText("복사").performClick()
    waitForIdle()

    assertEquals(1, copyCount)
    assertEquals(1, dismissCount)
  }

  @Test
  fun repasteOverlayInvokesRepasteAsText() = runComposeUiTest {
    val fake = FakeFfiEditor(onTick = { listOf(EditorEvent.StateChanged(emptyList())) })
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    try {
      setRepasteContent(editor)

      waitForIdle()
      onNodeWithText("서식 없이 다시 붙여넣기").performClick()

      waitUntil {
        fake.enqueued.any { message -> message == Message.Clipboard(ClipboardOp.RepasteAsText) }
      }
    } finally {
      scope.cancel()
      editor.dispose()
    }
  }

  @Test
  fun repasteOverlayShowsWithoutCursorWhenVisible() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    try {
      setRepasteContent(editor)

      waitForIdle()
      assertEquals(1, onAllNodesWithText("서식 없이 다시 붙여넣기").fetchSemanticsNodes().size)
    } finally {
      scope.cancel()
      editor.dispose()
    }
  }

  @Test
  fun repasteOverlayStaysNearTopWhenBottomIsOccluded() = runComposeUiTest {
    val fake = FakeFfiEditor()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    try {
      setRepasteContent(
        editor = editor,
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 400f, height = 700f),
            topInset = 24f,
            bottomOcclusionInset = 280f,
          ),
      )

      waitForIdle()
      val top = onNodeWithText("서식 없이 다시 붙여넣기").fetchSemanticsNode().boundsInRoot.top
      assertTrue(top < 120f, "expected top placement, got y=$top")
    } finally {
      scope.cancel()
      editor.dispose()
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setMenuContent(
    showCopyCutActions: Boolean = true,
    onCopy: () -> Unit = {},
    onCut: () -> Unit = {},
    onPaste: () -> Unit = {},
    onExpandWord: () -> Unit = {},
    onExpandSentence: () -> Unit = {},
    onExpandParagraph: () -> Unit = {},
    onSelectAll: () -> Unit = {},
    onDismiss: () -> Unit = {},
    availableExpansionUnits: Set<SelectionExpansionUnit> = SelectionExpansionUnit.entries.toSet(),
  ) {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorSelectionContextMenuOverlay(
          anchor = EditorContextMenuAnchor(centerX = 200f, above = 220f, below = 320f),
          overlaySize = Size(width = 400f, height = 700f),
          visibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
          showCopyCutActions = showCopyCutActions,
          availableExpansionUnits = availableExpansionUnits,
          onCopy = onCopy,
          onCut = onCut,
          onPaste = onPaste,
          onExpandWord = onExpandWord,
          onExpandSentence = onExpandSentence,
          onExpandParagraph = onExpandParagraph,
          onSelectAll = onSelectAll,
          onDismiss = onDismiss,
        )
      }
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setRepasteContent(
    editor: Editor,
    visibleArea: EditorVisibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
  ) {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorRepasteAsTextOverlay(
          editor = editor,
          visibleArea = visibleArea,
          visible = true,
          bringIntoViewRequests = EditorBringIntoViewRequests(),
        )
      }
    }
  }
}
