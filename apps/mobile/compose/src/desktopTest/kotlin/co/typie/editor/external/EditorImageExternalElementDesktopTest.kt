package co.typie.editor.external

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorImageExternalElementDesktopTest {
  @Test
  fun leavesImageAndPlaceholderHeightsToPublication() = runComposeUiTest {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake = FakeFfiEditor()
    val editor = Editor(fake, scope, Dispatchers.Unconfined)
    val runtime = EditorRuntime(scope)
    val externalState = EditorExternalElementState()
    val zoom = mutableFloatStateOf(1.05f)
    val element =
      ExternalElement(
        pageIdx = 0,
        node = "image",
        bounds = Rect(0f, 0f, 600f, 800f),
        isSelected = false,
        data = ExternalElementData.Image(id = "asset", proportion = 100, maxHeight = 800f),
      )
    fun reportedHeights() =
      fake.enqueued.filterIsInstance<Message.System>().mapNotNull {
        (it.event as? SystemEvent.SetExternalHeights)?.heights?.singleOrNull()?.height
      }

    try {
      runtime.attach(editor)
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(3f),
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalEditorRuntime provides runtime,
          LocalEditorUiState provides EditorUiState(),
          LocalEditorExternalElementState provides externalState,
        ) {
          Box(Modifier.requiredSize(1300.dp, 1800.dp)) {
            EditorExternalElementOverlay(listOf(element), displayZoom = zoom.floatValue)
          }
        }
      }
      waitForIdle()
      runOnIdle {
        assertEquals(emptyList(), reportedHeights())
        externalState.put(EditorImageAsset("asset", "", "", 600, 3000, 0.2, null))
      }
      waitForIdle()
      runOnIdle {
        assertEquals(emptyList(), reportedHeights())
        zoom.floatValue = 2f
        externalState.images.resizeDrafts["image"] =
          EditorImageResizeDraft(10f, imageResizeMaxSize(600f, 600f, 0.2f, 800f))
      }
      waitForIdle()
      runOnIdle {
        // Known image heights belong to publication, including live resize.
        assertEquals(emptyList(), reportedHeights())
        externalState.clear()
      }
      waitForIdle()
      runOnIdle { assertEquals(emptyList(), reportedHeights()) }
    } finally {
      runtime.clear()
      scope.cancel()
    }
  }
}
