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
  fun `continuous loading track includes both page margins inside the available width`() {
    assertEquals(
      640f,
      resolveEditorLoadingBodyTrackWidth(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        availableWidth = 720f,
      ),
    )
  }

  @Test
  fun `continuous loading track shrinks at the responsive boundary`() {
    assertEquals(
      620f,
      resolveEditorLoadingBodyTrackWidth(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        availableWidth = 620f,
      ),
    )
  }

  @Test
  fun `paginated loading body track stays at the page width in a wider viewport`() {
    assertEquals(
      720f,
      resolveEditorLoadingBodyTrackWidth(layoutSpec = paginatedLayout(), availableWidth = 960f),
    )
  }

  @Test
  fun `continuous loading body keeps the page padding`() {
    assertEquals(
      EditorLoadingBodyGeometry(trackWidth = 640f, leftPadding = 20f, rightPadding = 20f),
      resolveEditorLoadingBodyGeometry(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        availableWidth = 720f,
      ),
    )
  }

  @Test
  fun `paginated loading body uses the page content margins`() {
    assertEquals(
      EditorLoadingBodyGeometry(trackWidth = 720f, leftPadding = 40f, rightPadding = 80f),
      resolveEditorLoadingBodyGeometry(
        layoutSpec = paginatedLayout(pageMarginLeft = 40f, pageMarginRight = 80f),
        availableWidth = 960f,
      ),
    )
  }

  @Test
  fun `paginated loading body scales the page content margins with the fitted page`() {
    assertEquals(
      EditorLoadingBodyGeometry(trackWidth = 360f, leftPadding = 20f, rightPadding = 40f),
      resolveEditorLoadingBodyGeometry(
        layoutSpec = paginatedLayout(pageMarginLeft = 40f, pageMarginRight = 80f),
        availableWidth = 360f,
      ),
    )
  }

  @Test
  fun `loading track rejects invalid available widths`() {
    assertEquals(
      0f,
      resolveEditorLoadingBodyTrackWidth(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        availableWidth = Float.NaN,
      ),
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

  private fun paginatedLayout(pageMarginLeft: Float = 64f, pageMarginRight: Float = 64f) =
    EditorDocumentLayoutSpec.Paginated(
      pageWidth = 720f,
      pageHeight = 960f,
      pageMarginTop = 72f,
      pageMarginBottom = 72f,
      pageMarginLeft = pageMarginLeft,
      pageMarginRight = pageMarginRight,
    )
}
