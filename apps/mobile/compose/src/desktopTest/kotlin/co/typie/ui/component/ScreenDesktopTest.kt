package co.typie.ui.component

import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.LocalDialog
import kotlin.test.Test

private const val ContentTag = "screen-content"

@OptIn(ExperimentalTestApi::class)
class ScreenDesktopTest {
  @Test
  fun rendersWithoutNavigatorProviderWhenLoadableIsAbsent() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(LocalDialog provides Dialog()) {
        Screen { Box(Modifier.testTag(ContentTag)) }
      }
    }
    waitForIdle()

    onNodeWithTag(ContentTag).assertExists()
  }
}
