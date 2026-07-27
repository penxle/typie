package co.typie.ui.component.sheet

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlinx.coroutines.awaitCancellation

@OptIn(ExperimentalTestApi::class)
class SheetDismissCancellationDesktopTest {
  @Test
  fun dismissingTheSheetCancelsWorkStartedInsideIt() = runComposeUiTest {
    var started = false
    var cancelled = false

    setContent {
      val sheet = remember { Sheet() }

      LaunchedEffect(sheet) {
        sheet.present<Unit> {
          Column {
            Box(
              Modifier.testTag("start").size(48.dp).clickable {
                started = true
                try {
                  awaitCancellation()
                } finally {
                  cancelled = true
                }
              }
            )
            Box(Modifier.testTag("close").size(48.dp).clickable { dismiss() })
          }
        }
      }

      SheetOverlay(sheet)
    }

    waitForIdle()

    onNodeWithTag("start").performClick()
    waitUntil { started }

    onNodeWithTag("close").performClick()
    waitUntil { cancelled }

    assertTrue(cancelled)
  }
}
