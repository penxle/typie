package co.typie.screen.editor.editor

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

class EditorLoadingSkeletonTest {
  @Test
  fun `loading layout resolves continuous server json`() {
    val encoded = buildJsonObject {
      put("type", "continuous")
      put("maxWidth", 720)
    }

    assertEquals(
      EditorDocumentLayoutSpec.Continuous(maxWidth = 720f),
      resolveEditorLoadingLayoutSpec(encoded),
    )
  }

  @Test
  fun `loading layout resolves paginated server json`() {
    val encoded = buildJsonObject {
      put("type", "paginated")
      put("pageWidth", 794)
      put("pageHeight", 1123)
      put("pageMarginTop", 96)
      put("pageMarginBottom", 96)
      put("pageMarginLeft", 0)
      put("pageMarginRight", 80)
    }

    assertEquals(
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 794f,
        pageHeight = 1123f,
        pageMarginTop = 96f,
        pageMarginBottom = 96f,
        pageMarginLeft = 0f,
        pageMarginRight = 80f,
      ),
      resolveEditorLoadingLayoutSpec(encoded),
    )
  }

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
