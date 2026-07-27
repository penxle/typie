package co.typie.editor

import co.typie.editor.ffi.LayoutMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorPagePresetsTest {
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

  private fun optionsOf(layout: LayoutMode.Paginated) = pageMarginOptionsOf(layout, emptyList())

  @Test
  fun `page size option is matched on width and height`() {
    assertEquals(a4, pageLayoutOptionOf(layoutOf(a4, "narrow")))
    assertEquals(b6, pageLayoutOptionOf(layoutOf(b6, "wide")))
    assertNull(pageLayoutOptionOf(layoutOf(a4, "narrow").copy(pageWidth = 500)))
    assertNull(pageLayoutOptionOf(layoutOf(a4, "narrow").copy(pageHeight = 700)))
  }

  @Test
  fun `page size preset falls back to custom`() {
    assertEquals("a4", pageSizePresetOf(layoutOf(a4, "normal")))
    assertEquals("b5", pageSizePresetOf(layoutOf(b5, "wide")))
    assertEquals(PagePresetCustom, pageSizePresetOf(layoutOf(a4, "normal").copy(pageWidth = 500)))
  }

  @Test
  fun `margin preset is read from the page size table`() {
    assertEquals(b5.margins, optionsOf(layoutOf(b5, "normal")))
    assertEquals(
      "normal",
      pageMarginPresetOf(layoutOf(b5, "normal"), optionsOf(layoutOf(b5, "normal"))),
    )
    assertEquals("wide", pageMarginPresetOf(layoutOf(b6, "wide"), optionsOf(layoutOf(b6, "wide"))))
    assertEquals("narrow", pageMarginPresetOf(layoutOf(b5, "normal"), a4.margins))
  }

  @Test
  fun `margin preset falls back to custom`() {
    val layout = layoutOf(b5, "normal").copy(pageMarginTop = 7)

    assertEquals(PagePresetCustom, pageMarginPresetOf(layout, optionsOf(layout)))
  }

  @Test
  fun `applying a margin preset uses the page size table`() {
    val layout = layoutOf(b6, "normal")

    assertEquals(layoutOf(b6, "wide"), applyMarginPreset(layout, "wide", optionsOf(layout)))
    assertEquals(57, applyMarginPreset(layout, "wide", optionsOf(layout)).pageMarginTop)
  }

  @Test
  fun `changing the page size remaps the named margin to the new table`() {
    val from = layoutOf(a4, "wide")
    val to = applyPageSizePreset(from, "b6", optionsOf(from))

    assertEquals(layoutOf(b6, "wide"), to)
    assertEquals("wide", pageMarginPresetOf(to, optionsOf(to)))
  }

  @Test
  fun `changing the page size keeps unnamed margins`() {
    val from = layoutOf(a4, "normal").copy(pageMarginTop = 7)
    val to = applyPageSizePreset(from, "b6", optionsOf(from))

    assertEquals(b6.layout.pageWidth, to.pageWidth)
    assertEquals(b6.layout.pageHeight, to.pageHeight)
    assertEquals(7, to.pageMarginTop)
    assertEquals(94, to.pageMarginBottom)
  }

  @Test
  fun `custom page sizes have no margin table without a fallback`() {
    val custom = layoutOf(a4, "narrow").copy(pageWidth = 500, pageHeight = 700)

    assertEquals(emptyList(), optionsOf(custom))
    assertEquals(PagePresetCustom, pageMarginPresetOf(custom, optionsOf(custom)))
    assertEquals(custom, applyMarginPreset(custom, "wide", optionsOf(custom)))
    assertEquals(
      custom.copy(pageWidth = b6.layout.pageWidth, pageHeight = b6.layout.pageHeight),
      applyPageSizePreset(custom, "b6", optionsOf(custom)),
    )
  }

  @Test
  fun `custom page sizes use the fallback margin table`() {
    val custom = layoutOf(a4, "narrow").copy(pageWidth = 500, pageHeight = 700)
    val options = pageMarginOptionsOf(custom, a4.margins)

    assertEquals(a4.margins, options)
    assertEquals("narrow", pageMarginPresetOf(custom, options))
    assertEquals(
      LayoutMode.Paginated(
        pageWidth = 500,
        pageHeight = 700,
        pageMarginTop = 132,
        pageMarginBottom = 132,
        pageMarginLeft = 132,
        pageMarginRight = 132,
      ),
      applyMarginPreset(custom, "wide", options),
    )
    assertEquals(layoutOf(b6, "narrow"), applyPageSizePreset(custom, "b6", options))
  }

  @Test
  fun `presets never clamp the margins`() {
    val tiny = LayoutMode.Paginated(200, 200, 0, 0, 0, 0)
    val applied = applyMarginPreset(tiny, "wide", a4.margins)

    assertEquals(132, applied.pageMarginLeft)
    assertEquals(132, applied.pageMarginRight)
    assertTrue(applied.pageMarginLeft + applied.pageMarginRight > applied.pageWidth)
  }

  @Test
  fun `unknown preset names change nothing`() {
    val layout = layoutOf(a4, "normal")

    assertEquals(layout, applyMarginPreset(layout, "huge", optionsOf(layout)))
    assertEquals(layout, applyPageSizePreset(layout, "a3", optionsOf(layout)))
  }
}
