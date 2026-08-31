package co.typie.screen.document.document

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.ProvideDesktopDebugKeyboardPresentation
import co.typie.ui.component.popover.LocalPopoverOverlayState
import co.typie.ui.component.popover.PopoverOverlay
import co.typie.ui.component.popover.PopoverOverlayState
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.component.sheet.SheetOverlay
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.Toast
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class DocumentExportSheetDesktopTest {
  private fun ComposeUiTest.presentExportSheet() {
    val sheet = Sheet()
    val popoverOverlayState = PopoverOverlayState()
    val model = DocumentViewModel()

    setContent {
      ProvideDesktopDebugKeyboardPresentation {
        CompositionLocalProvider(
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalHazeBlurStyle provides
            HazeBlurStyle {
              blurRadius(20.dp)
              noiseFactor(0f)
              colorEffects(emptyList())
            },
          LocalToast provides Toast(),
          LocalPopoverOverlayState provides popoverOverlayState,
        ) {
          Box(Modifier.size(width = 400.dp, height = 800.dp)) {
            LaunchedEffect(Unit) {
              sheet.present {
                DocumentExportSheet(model = model, documentId = "doc-1", documentLayout = null)
              }
            }
            SheetOverlay(sheet)
            PopoverOverlay(popoverOverlayState)
          }
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
    waitForIdle()
  }

  @Test
  fun format_selector_anchor_sits_at_the_row_trailing_edge() = runComposeUiTest {
    presentExportSheet()

    val anchorBounds = onNodeWithText("PDF (Acrobat)").fetchSemanticsNode().boundsInRoot
    val windowWidthPx = with(density) { 400.dp.toPx() }

    assertTrue(
      anchorBounds.right > windowWidthPx * 0.7f,
      "expected the format anchor's right edge (${anchorBounds.right}) to sit near the " +
        "sheet's trailing edge ($windowWidthPx), proving the popover gets a wide placement",
    )
  }

  @Test
  fun custom_mode_is_idempotent_and_scoped_to_its_own_row() = runComposeUiTest {
    presentExportSheet()

    val customChips = onAllNodesWithText("사용자 정의")
    customChips[0].performClick()
    waitForIdle()

    onNodeWithText("가로").assertExists()
    onNodeWithText("세로").assertExists()
    onNodeWithText("상").assertDoesNotExist()

    onAllNodesWithText("사용자 정의")[0].performClick()
    waitForIdle()
    onNodeWithText("가로").assertExists()

    onAllNodesWithText("사용자 정의")[1].performClick()
    waitForIdle()

    onNodeWithText("상").assertExists()
    onNodeWithText("하").assertExists()
    onNodeWithText("좌").assertExists()
    onNodeWithText("우").assertExists()
    onNodeWithText("가로").assertExists()
    onNodeWithText("세로").assertExists()

    onNodeWithText("A5").performClick()
    waitForIdle()

    onNodeWithText("가로").assertDoesNotExist()
    onNodeWithText("세로").assertDoesNotExist()
    onNodeWithText("상").assertExists()
  }

  @Test
  fun custom_page_width_empties_the_margin_options_without_reopening_margin_custom_mode() =
    runComposeUiTest {
      presentExportSheet()

      onAllNodesWithText("사용자 정의")[0].performClick()
      waitForIdle()

      onNodeWithText("210").performTextReplacement("200")
      waitForIdle()
      onNodeWithText("200").performImeAction()
      waitForIdle()

      onNodeWithText("좁게").assertDoesNotExist()
      onNodeWithText("보통").assertDoesNotExist()
      onNodeWithText("넓게").assertDoesNotExist()
      onAllNodesWithText("사용자 정의").assertCountEquals(2)
    }
}
