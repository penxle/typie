package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class EditorLongPressSemanticTest {
  @Test
  fun `reading resolves word selection on every shared platform`() = runTest {
    val editor = editor()

    Platform.entries.forEach { platform ->
      assertEquals(
        EditorLongPressSemanticIntent.WordSelection,
        EditorLongPressSemantic()
          .resolveIntent(editor = editor, point = Point, platform = platform, editing = false),
        platform.name,
      )
    }
  }

  @Test
  fun `editing preserves the native platform matrix`() = runTest {
    val ordinaryEditor = editor()

    assertEquals(
      EditorLongPressSemanticIntent.WordSelection,
      EditorLongPressSemantic()
        .resolveIntent(
          editor = ordinaryEditor,
          point = Point,
          platform = Platform.Android,
          editing = true,
        ),
    )
    assertEquals(
      EditorLongPressSemanticIntent.CursorMove,
      EditorLongPressSemantic()
        .resolveIntent(
          editor = ordinaryEditor,
          point = Point,
          platform = Platform.iOS,
          editing = true,
        ),
    )
    assertEquals(
      EditorLongPressSemanticIntent.CursorMove,
      EditorLongPressSemantic()
        .resolveIntent(
          editor = ordinaryEditor,
          point = Point,
          platform = Platform.Desktop,
          editing = true,
        ),
    )

    val androidCursorEditor = editor(cursorHit = true)
    assertEquals(
      EditorLongPressSemanticIntent.CursorMove,
      EditorLongPressSemantic()
        .resolveIntent(
          editor = androidCursorEditor,
          point = Point,
          platform = Platform.Android,
          editing = true,
        ),
    )
  }

  private suspend fun kotlinx.coroutines.test.TestScope.editor(cursorHit: Boolean = false): Editor {
    val editor =
      Editor(
        FakeFfiEditor(
          cursorHitRectsProvider = {
            if (cursorHit) FakeFfiEditor.coveringHitRects(0) else emptyList()
          }
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    editor.sync {}
    return editor
  }

  private companion object {
    val Point = PagePoint(page = 0, x = 10f, y = 20f)
  }
}
