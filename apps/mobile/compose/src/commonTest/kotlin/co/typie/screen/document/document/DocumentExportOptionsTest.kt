package co.typie.screen.document.document

import co.typie.editor.EditorPageLayoutOption
import co.typie.editor.EditorValues
import co.typie.editor.PagePresetCustom
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.LayoutMode
import co.typie.graphql.type.DocumentExportFormat
import co.typie.ui.component.editorsettings.MinContentSizePx
import co.typie.ui.component.editorsettings.clampPageMargins
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DocumentExportOptionsTest {
  private val a4 = EditorValues.pageLayout.first { it.value == "a4" }
  private val b5 = EditorValues.pageLayout.first { it.value == "b5" }
  private val b6 = EditorValues.pageLayout.first { it.value == "b6" }

  private fun layoutOf(option: EditorPageLayoutOption, margin: String) =
    option.margins
      .first { it.value == margin }
      .let {
        LayoutMode.Paginated(
          pageWidth = option.layout.pageWidth,
          pageHeight = option.layout.pageHeight,
          pageMarginTop = it.top,
          pageMarginBottom = it.bottom,
          pageMarginLeft = it.left,
          pageMarginRight = it.right,
        )
      }

  @Test
  fun `continuous document starts from the a4 preset`() {
    val initial = exportInitialLayout(EditorDocumentLayoutSpec.Continuous(maxWidth = 600f))

    assertEquals(a4.layout.pageWidth, initial.pageWidth)
    assertEquals(a4.layout.pageHeight, initial.pageHeight)
    assertEquals("normal", exportMarginPreset(initial))
  }

  @Test
  fun `missing layout starts from the a4 preset`() {
    assertEquals(
      exportInitialLayout(EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)),
      exportInitialLayout(null),
    )
  }

  @Test
  fun `paginated document starts from its own layout`() {
    val spec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 500.4f,
        pageHeight = 700.6f,
        pageMarginTop = 10f,
        pageMarginBottom = 11f,
        pageMarginLeft = 12f,
        pageMarginRight = 13f,
      )

    assertEquals(
      LayoutMode.Paginated(
        pageWidth = 500,
        pageHeight = 701,
        pageMarginTop = 10,
        pageMarginBottom = 11,
        pageMarginLeft = 12,
        pageMarginRight = 13,
      ),
      exportInitialLayout(spec),
    )
  }

  @Test
  fun `only paginated documents offer the current layout toggle`() {
    assertTrue(
      exportCanUseDocumentLayout(EditorDocumentLayoutSpec.Paginated(1f, 1f, 0f, 0f, 0f, 0f))
    )
    assertFalse(exportCanUseDocumentLayout(EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)))
    assertFalse(exportCanUseDocumentLayout(null))
  }

  @Test
  fun `presets are detected and fall back to custom`() {
    assertEquals("a4", exportPageSizePreset(layoutOf(a4, "normal")))
    assertEquals("normal", exportMarginPreset(layoutOf(a4, "normal")))
    assertEquals("narrow", exportMarginPreset(layoutOf(a4, "narrow")))

    assertEquals("b5", exportPageSizePreset(layoutOf(b5, "normal")))
    assertEquals("normal", exportMarginPreset(layoutOf(b5, "normal")))
    assertEquals("wide", exportMarginPreset(layoutOf(b6, "wide")))

    val custom = layoutOf(a4, "normal").copy(pageWidth = 500, pageMarginTop = 3)
    assertEquals(PagePresetCustom, exportPageSizePreset(custom))
    assertEquals(PagePresetCustom, exportMarginPreset(custom))
  }

  @Test
  fun `changing the page size remaps the named margin to the new page size`() {
    val changed = exportApplyPageSizePreset(layoutOf(a4, "narrow"), "b5")

    assertEquals(b5.layout.pageWidth, changed.pageWidth)
    assertEquals(b5.layout.pageHeight, changed.pageHeight)
    assertEquals("narrow", exportMarginPreset(changed))
    assertEquals(layoutOf(b5, "narrow"), changed)
  }

  @Test
  fun `changing the page size keeps custom margins as they are`() {
    val custom = layoutOf(a4, "normal").copy(pageMarginTop = 7)
    val changed = exportApplyPageSizePreset(custom, "b5")

    assertEquals(b5.layout.pageWidth, changed.pageWidth)
    assertEquals(7, changed.pageMarginTop)
  }

  @Test
  fun `custom page sizes offer no margin presets`() {
    val custom = layoutOf(a4, "normal").copy(pageWidth = 500, pageHeight = 700)

    assertEquals(PagePresetCustom, exportPageSizePreset(custom))
    assertEquals(emptyList(), exportMarginOptions(custom))
    assertEquals(PagePresetCustom, exportMarginPreset(custom))
    assertEquals(custom, exportApplyMarginPreset(custom, "narrow"))
  }

  @Test
  fun `naming the page size brings the margin presets back`() {
    val custom = layoutOf(a4, "normal").copy(pageWidth = 500, pageHeight = 700)
    val named = exportApplyPageSizePreset(custom, "b6")

    assertEquals("b6", exportPageSizePreset(named))
    assertEquals(b6.margins, exportMarginOptions(named))
    assertEquals(layoutOf(b6, "wide"), exportApplyMarginPreset(named, "wide"))
  }

  @Test
  fun `margin presets come from the selected page size`() {
    val wide = b6.margins.first { it.value == "wide" }
    val applied = exportApplyMarginPreset(layoutOf(b6, "normal"), "wide")

    assertEquals(wide.top, applied.pageMarginTop)
    assertEquals(wide.left, applied.pageMarginLeft)
    assertEquals("wide", exportMarginPreset(applied))
  }

  @Test
  fun `shrinking the page size keeps the content minimum size`() {
    val oversized = layoutOf(a4, "normal").copy(pageMarginLeft = 400, pageMarginRight = 400)
    val applied = exportApplyPageSizePreset(oversized, "b6")

    assertEquals(b6.layout.pageWidth, applied.pageWidth)
    assertTrue(
      applied.pageMarginLeft + applied.pageMarginRight <= applied.pageWidth - MinContentSizePx
    )
  }

  @Test
  fun `margins are clamped so the content keeps its minimum size`() {
    val overflowing = layoutOf(a4, "normal").copy(pageMarginLeft = a4.layout.pageWidth)
    val clamped = clampPageMargins(overflowing)

    assertTrue(
      clamped.pageMarginLeft + clamped.pageMarginRight <= a4.layout.pageWidth - MinContentSizePx
    )
  }

  @Test
  fun `epub sends no layout and disables layout controls`() {
    val layout = layoutOf(a4, "normal")

    assertNull(exportLayoutInput(DocumentExportFormat.EPUB, layout))
    assertFalse(exportLayoutControlsEnabled(DocumentExportFormat.EPUB, false))
  }

  @Test
  fun `other formats send the layout as px integers`() {
    val layout = layoutOf(a4, "narrow")
    val input = exportLayoutInput(DocumentExportFormat.PDF, layout)

    assertEquals(layout.pageWidth, input?.pageWidth)
    assertEquals(layout.pageHeight, input?.pageHeight)
    assertEquals(layout.pageMarginTop, input?.pageMarginTop)
    assertEquals(layout.pageMarginBottom, input?.pageMarginBottom)
    assertEquals(layout.pageMarginLeft, input?.pageMarginLeft)
    assertEquals(layout.pageMarginRight, input?.pageMarginRight)
  }

  @Test
  fun `using the document layout disables layout controls`() {
    assertFalse(exportLayoutControlsEnabled(DocumentExportFormat.PDF, true))
    assertTrue(exportLayoutControlsEnabled(DocumentExportFormat.PDF, false))
  }

  @Test
  fun `progress notice appears at fifteen and sixty seconds`() {
    assertNull(exportProgressNotice(0))
    assertNull(exportProgressNotice(14))
    assertEquals("문서가 길면 1분 이상 걸릴 수 있어요.", exportProgressNotice(15))
    assertEquals("문서가 길면 1분 이상 걸릴 수 있어요.", exportProgressNotice(59))
    assertEquals("계속 진행 중이에요. 앱을 닫지 말아 주세요.", exportProgressNotice(60))
    assertEquals("계속 진행 중이에요. 앱을 닫지 말아 주세요.", exportProgressNotice(300))
  }
}
