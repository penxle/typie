package co.typie.screen.editor.editor.layout

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveEditorBodyFillHeight
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorBodyGeometryTest {
  @Test
  fun `geometry respects visible occlusion when resolving body height and bottom padding`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = EditorMeasuredSize(width = 720f, height = 900f),
            headerHeight = 180f,
            topInset = 120f,
            imeInset = 100f,
            toolbarTop = 756f,
          ),
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = listOf(Size(width = 600f, height = 800f)),
      )

    assertEquals(600f, geometry.pageColumnWidth)
    assertEquals(576f, geometry.minimumBodyHeight)
    assertEquals(40f, geometry.defaultTopPadding)
    assertEquals(184f, geometry.defaultBottomPadding)
    assertEquals(184f, geometry.activeBottomPadding)
  }

  @Test
  fun `geometry falls back to the visible width before page metrics arrive`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = EditorMeasuredSize(width = 360f, height = 640f),
            headerHeight = 72f,
            topInset = 72f,
          ),
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = emptyList(),
      )

    assertEquals(360f, geometry.pageColumnWidth)
    assertEquals(568f, geometry.minimumBodyHeight)
    assertEquals(40f, geometry.defaultTopPadding)
    assertEquals(40f, geometry.defaultBottomPadding)
    assertEquals(40f, geometry.activeBottomPadding)
  }

  @Test
  fun `geometry reserves only the keep-visible cursor margin when bottom occlusion is absent`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = EditorMeasuredSize(width = 360f, height = 640f),
            headerHeight = 72f,
            topInset = 72f,
          ),
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = emptyList(),
      )

    assertEquals(40f, geometry.defaultBottomPadding)
    assertEquals(40f, geometry.activeBottomPadding)
  }

  @Test
  fun `minimum body height is clamped to zero`() {
    assertEquals(
      0f,
      resolveMinimumBodyHeight(viewportHeight = 400f, headerHeight = 320f, bottomOcclusion = 120f),
    )
  }

  @Test
  fun `body fill height covers the remaining viewport below the core track`() {
    assertEquals(312f, resolveEditorBodyFillHeight(minimumHeight = 400f, coreTrackHeight = 88f))
    assertEquals(0f, resolveEditorBodyFillHeight(minimumHeight = 400f, coreTrackHeight = 420f))
  }

  @Test
  fun `geometry prefers the document layout spec over engine page widths`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = EditorMeasuredSize(width = 960f, height = 900f),
            headerHeight = 120f,
            topInset = 120f,
          ),
        layoutSpec = EditorDocumentLayoutSpec.Paginated(pageWidth = 720f),
        pageSizes = listOf(Size(width = 700f, height = 960f)),
      )

    assertEquals(720f, geometry.pageColumnWidth)
    assertEquals(20f, geometry.activeBottomPadding)
  }
}
