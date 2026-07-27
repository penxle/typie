package co.typie.screen.editor.editor

import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorLoadingSkeletonTest {
  @Test
  fun `ready geometry requires an attached editor with finite positive pages and track`() {
    assertEquals(
      false,
      hasValidEditorGeometry(
        editorAttached = false,
        pageSizes = listOf(Size(width = 640f, height = 800f)),
        trackWidth = 640f,
      ),
    )
    assertEquals(
      false,
      hasValidEditorGeometry(
        editorAttached = true,
        pageSizes = emptyList<Size>(),
        trackWidth = 640f,
      ),
    )
    assertEquals(
      false,
      hasValidEditorGeometry(
        editorAttached = true,
        pageSizes = listOf(Size(width = 640f, height = Float.NaN)),
        trackWidth = 640f,
      ),
    )
    assertEquals(
      true,
      hasValidEditorGeometry(
        editorAttached = true,
        pageSizes = listOf(Size(width = 640f, height = 800f)),
        trackWidth = 640f,
      ),
    )
  }
}
