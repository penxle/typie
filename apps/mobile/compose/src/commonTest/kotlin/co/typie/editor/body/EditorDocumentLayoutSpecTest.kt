package co.typie.editor.body

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

class EditorDocumentLayoutSpecTest {
  @Test
  fun `decodes continuous server json`() {
    val encoded = buildJsonObject {
      put("type", "continuous")
      put("maxWidth", 720)
    }

    assertEquals(
      EditorDocumentLayoutSpec.Continuous(maxWidth = 720f),
      decodeDocumentLayoutSpec(encoded),
    )
  }

  @Test
  fun `decodes paginated server json`() {
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
      decodeDocumentLayoutSpec(encoded),
    )
  }

  @Test
  fun `returns null for non object values`() {
    assertNull(decodeDocumentLayoutSpec(null))
    assertNull(decodeDocumentLayoutSpec(JsonNull))
  }

  @Test
  fun `returns null when paginated json misses a field`() {
    val encoded = buildJsonObject {
      put("type", "paginated")
      put("pageWidth", 794)
    }

    assertNull(decodeDocumentLayoutSpec(encoded))
  }
}
