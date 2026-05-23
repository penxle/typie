package co.typie.editor.interaction

import co.typie.editor.EditorState
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.ext.ScrollGestureLockState
import kotlin.test.Test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorInteractionScopeTest {
  @Test
  fun `editor state observation is ignored before editor attaches`() =
    runTest(StandardTestDispatcher()) {
      val scope = EditorInteractionScope(coroutineScope = this)

      scope.update(
        editor = null,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = EditorUiState(),
        density = 1f,
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        onSelectionHaptic = {},
      )

      scope.onEditorStateChanged(EditorState.Initial)
    }
}
