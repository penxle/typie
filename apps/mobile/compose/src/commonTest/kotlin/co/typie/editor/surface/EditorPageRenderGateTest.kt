package co.typie.editor.surface

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorPageRenderGateTest {
  @Test
  fun `page inside viewport is active`() {
    assertTrue(isEditorPageRenderActive(pageTop = 100f, pageBottom = 900f, rootHeight = 1000f))
  }

  @Test
  fun `page spanning entire viewport is active`() {
    assertTrue(isEditorPageRenderActive(pageTop = -500f, pageBottom = 1500f, rootHeight = 1000f))
  }

  @Test
  fun `page below viewport within overscan is active`() {
    assertTrue(isEditorPageRenderActive(pageTop = 1500f, pageBottom = 2400f, rootHeight = 1000f))
  }

  @Test
  fun `page below viewport beyond overscan is inactive`() {
    assertFalse(isEditorPageRenderActive(pageTop = 2000f, pageBottom = 2900f, rootHeight = 1000f))
  }

  @Test
  fun `page above viewport within overscan is active`() {
    assertTrue(isEditorPageRenderActive(pageTop = -1400f, pageBottom = -500f, rootHeight = 1000f))
  }

  @Test
  fun `page above viewport beyond overscan is inactive`() {
    assertFalse(isEditorPageRenderActive(pageTop = -2900f, pageBottom = -2000f, rootHeight = 1000f))
  }

  @Test
  fun `page touching overscan boundary below is inactive`() {
    assertFalse(isEditorPageRenderActive(pageTop = 2000f, pageBottom = 2900f, rootHeight = 1000f))
    assertTrue(isEditorPageRenderActive(pageTop = 1999f, pageBottom = 2900f, rootHeight = 1000f))
  }

  @Test
  fun `page touching overscan boundary above is inactive`() {
    assertFalse(isEditorPageRenderActive(pageTop = -1900f, pageBottom = -1000f, rootHeight = 1000f))
    assertTrue(isEditorPageRenderActive(pageTop = -1900f, pageBottom = -999f, rootHeight = 1000f))
  }

  @Test
  fun `unmeasured root fails open`() {
    assertTrue(isEditorPageRenderActive(pageTop = 99999f, pageBottom = 100999f, rootHeight = 0f))
    assertTrue(isEditorPageRenderActive(pageTop = 99999f, pageBottom = 100999f, rootHeight = -1f))
  }
}
