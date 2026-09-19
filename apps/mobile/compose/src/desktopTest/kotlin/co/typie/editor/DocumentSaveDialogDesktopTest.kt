package co.typie.editor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.ProvideDesktopDebugKeyboardPresentation
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogOverlay
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlinx.coroutines.launch

@OptIn(ExperimentalTestApi::class)
class DocumentSaveDialogDesktopTest {
  @Test
  fun recoveryKeepsTheSameDialogAndContinuesAfterFiveSeconds() = runComposeUiTest {
    val dialog = Dialog()
    val saveState = mutableStateOf(DocumentSaveState.Pending)
    var result: DialogResult<Unit>? = null
    mainClock.autoAdvance = false
    showSaveDialog(dialog, saveState) { result = it }
    val entry = dialog.current
    assertNotNull(entry)
    onNodeWithText("저장하지 않고 나가기").assertExists()

    runOnIdle { saveState.value = DocumentSaveState.Failed }
    mainClock.advanceTimeByFrame()
    waitForIdle()
    assertSame(entry, dialog.current)
    mainClock.advanceTimeBy(6_000)
    assertNull(result)

    runOnIdle { saveState.value = DocumentSaveState.Pending }
    mainClock.advanceTimeByFrame()
    waitForIdle()
    assertSame(entry, dialog.current)

    runOnIdle { saveState.value = DocumentSaveState.Protected }
    mainClock.advanceTimeByFrame()
    waitForIdle()
    assertSame(entry, dialog.current)
    onNodeWithText("5", useUnmergedTree = true).assertExists()
    onNodeWithText("저장하지 않고 나가기").assertDoesNotExist()

    mainClock.advanceTimeBy(4_900)
    waitForIdle()
    assertNull(result)
    onNodeWithText("1", useUnmergedTree = true).assertExists()
    mainClock.advanceTimeBy(400)
    waitForIdle()
    assertEquals(DialogResult.Resolved(Unit), result)
    assertNull(dialog.current)
  }

  @Test
  fun continueEditingCancelsTheCompletedDepartureWithoutLaterReplay() = runComposeUiTest {
    val dialog = Dialog()
    val saveState = mutableStateOf(DocumentSaveState.Protected)
    var result: DialogResult<Unit>? = null
    mainClock.autoAdvance = false
    showSaveDialog(dialog, saveState) { result = it }

    onNodeWithText("계속 편집").performClick()
    mainClock.advanceTimeBy(300)
    waitForIdle()
    assertEquals(DialogResult.Dismissed, result)
    mainClock.advanceTimeBy(6_000)
    waitForIdle()
    assertEquals(DialogResult.Dismissed, result)
    assertNull(dialog.current)
  }

  @Test
  fun requiredReloadReplacesRetryAndDiscardWithTheCompletedReloadAction() = runComposeUiTest {
    val dialog = Dialog()
    val saveState = mutableStateOf(DocumentSaveState.Failed)
    var result: DialogResult<Unit>? = null
    mainClock.autoAdvance = false
    showSaveDialog(dialog, saveState, reload = true) { result = it }
    assertFalse(checkNotNull(dialog.current).dismissible)
    onNodeWithText("다시 시도").assertExists()
    onNodeWithText("변경사항 버리고 불러오기").assertExists()

    runOnIdle { saveState.value = DocumentSaveState.Protected }
    mainClock.advanceTimeByFrame()
    waitForIdle()
    onNodeWithText("다시 시도").assertDoesNotExist()
    onNodeWithText("변경사항 버리고 불러오기").assertDoesNotExist()
    onNodeWithText("계속 편집").assertDoesNotExist()
    onNodeWithText("불러오기").performClick()
    mainClock.advanceTimeBy(300)
    waitForIdle()
    assertEquals(DialogResult.Resolved(Unit), result)
  }

  @Test
  fun countdownDoesNotElapseWhileAnotherDialogIsInFront() = runComposeUiTest {
    val dialog = Dialog()
    val saveState = mutableStateOf(DocumentSaveState.Protected)
    var result: DialogResult<Unit>? = null
    mainClock.autoAdvance = false
    setContent {
      ProvideDesktopDebugKeyboardPresentation {
        CompositionLocalProvider(LocalThemeMode provides ResolvedThemeMode.Light) {
          Box(Modifier.size(400.dp, 700.dp)) {
            LaunchedEffect(Unit) {
              launch { dialog.present<Unit> { Text("앞선 확인창") } }
              launch { result = dialog.confirmDocumentSave(saveState) }
            }
            DialogOverlay(dialog)
          }
        }
      }
    }
    mainClock.advanceTimeBy(10_000)
    waitForIdle()
    assertEquals(2, dialog.queue.size)
    assertNull(result)

    runOnIdle { dialog.resolveCurrentEntry(DialogResult.Dismissed) }
    mainClock.advanceTimeBy(300)
    waitForIdle()
    onNodeWithText("5", useUnmergedTree = true).assertExists()
    assertNull(result)
    mainClock.advanceTimeBy(5_000)
    waitForIdle()
    assertEquals(DialogResult.Resolved(Unit), result)
  }

  private fun ComposeUiTest.showSaveDialog(
    dialog: Dialog,
    saveState: State<DocumentSaveState>,
    reload: Boolean = false,
    onResult: (DialogResult<Unit>) -> Unit,
  ) {
    setContent {
      ProvideDesktopDebugKeyboardPresentation {
        CompositionLocalProvider(LocalThemeMode provides ResolvedThemeMode.Light) {
          Box(Modifier.size(400.dp, 700.dp)) {
            LaunchedEffect(Unit) { onResult(dialog.confirmDocumentSave(saveState, reload)) }
            DialogOverlay(dialog)
          }
        }
      }
    }
    mainClock.advanceTimeBy(300)
    waitForIdle()
  }
}
