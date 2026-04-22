package co.typie.screen.editor.editor.zoom

import co.typie.editor.EditorViewportAnchor
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class EditorZoomViewportSyncTest {
  @Test
  fun `zoom viewport target keeps first-page anchor under focal point`() {
    val target =
      resolveZoomViewportScrollTarget(
        anchor = EditorViewportAnchor(page = 0, x = 120f, y = 200f),
        focalX = 80f,
        focalY = 150f,
        displayZoom = 1.5f,
        currentHorizontalScroll = 20f,
        currentVerticalScroll = 100f,
        pageSizes = listOf(Size(width = 720f, height = 960f)),
      )

    assertNotNull(target)
    assertEquals(120f, target.horizontalScroll, 0.0001f)
    assertEquals(250f, target.verticalScroll, 0.0001f)
  }

  @Test
  fun `zoom viewport target accumulates previous page heights and gaps`() {
    val target =
      resolveZoomViewportScrollTarget(
        anchor = EditorViewportAnchor(page = 2, x = 32f, y = 48f),
        focalX = 24f,
        focalY = 40f,
        displayZoom = 1.25f,
        currentHorizontalScroll = 12f,
        currentVerticalScroll = 180f,
        pageSizes =
          listOf(
            Size(width = 720f, height = 800f),
            Size(width = 720f, height = 900f),
            Size(width = 720f, height = 1000f),
          ),
      )

    assertNotNull(target)
    assertEquals(28f, target.horizontalScroll, 0.0001f)
    assertEquals(2385f, target.verticalScroll, 0.0001f)
  }

  @Test
  fun `zoom viewport target is null when anchor page is unavailable`() {
    val target =
      resolveZoomViewportScrollTarget(
        anchor = EditorViewportAnchor(page = 1, x = 0f, y = 0f),
        focalX = 0f,
        focalY = 0f,
        displayZoom = 1f,
        currentHorizontalScroll = 0f,
        currentVerticalScroll = 0f,
        pageSizes = listOf(Size(width = 720f, height = 960f)),
      )

    assertNull(target)
  }
}
