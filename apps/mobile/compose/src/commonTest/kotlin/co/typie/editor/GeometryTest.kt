package co.typie.editor

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class GeometryTest {

  private val sizes =
    listOf(
      Size(width = 400f, height = 600f),
      Size(width = 400f, height = 800f),
      Size(width = 400f, height = 500f),
    )
  private val offsets = mapOf(0 to Offset(0f, 0f), 1 to Offset(0f, 600f), 2 to Offset(0f, 1400f))

  private val offsetsWithGap =
    mapOf(0 to Offset(0f, 0f), 1 to Offset(0f, 620f), 2 to Offset(0f, 1440f))

  private val offsetsCentered = mapOf(0 to Offset(50f, 0f), 1 to Offset(50f, 600f))
  private val sizesCentered =
    listOf(Size(width = 300f, height = 600f), Size(width = 300f, height = 400f))

  @Test
  fun unionRects_returns_the_union_of_all_valid_rects() {
    val rect =
      unionRects(
        listOf(
          Rect(left = 40f, top = 420f, right = 70f, bottom = 450f),
          Rect(left = 30f, top = 380f, right = 60f, bottom = 410f),
          Rect(left = 80f, top = 460f, right = 20f, bottom = 500f),
        )
      )

    assertEquals(Rect(left = 20f, top = 380f, right = 80f, bottom = 500f), rect)
  }

  @Test
  fun unionRects_returns_null_when_no_valid_rects_are_present() {
    assertNull(unionRects(emptyList()))
    assertNull(unionRects(listOf(Rect(left = Float.NaN, top = 0f, right = 10f, bottom = 20f))))
  }

  @Test
  fun pageRectsToContentRect_returns_the_content_union_across_pages() {
    val rect =
      pageRectsToContentRect(
        rects =
          listOf(
            PageRect(pageIdx = 0, rect = FfiRect(x = 20f, y = 50f, width = 10f, height = 10f)),
            PageRect(pageIdx = 1, rect = FfiRect(x = 3f, y = 20f, width = 5f, height = 5f)),
          ),
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 100f,
            pageHeight = 100f,
            pageMarginTop = 0f,
            pageMarginBottom = 0f,
            pageMarginLeft = 0f,
            pageMarginRight = 0f,
          ),
        pageSizes = listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f)),
        contentOriginX = 5f,
        contentOriginY = 10f,
      )

    assertEquals(Rect(left = 8f, top = 60f, right = 35f, bottom = 159f), rect)
  }

  @Test
  fun localToGlobal_adds_page_offset() {
    val result = EditorViewportTransform(pageOffsets = offsets).localToGlobal(1, 100f, 50f)
    assertNotNull(result)
    assertEquals(100f, result.x)
    assertEquals(650f, result.y)
  }

  @Test
  fun localToGlobal_scales_page_local_coordinates_with_display_zoom() {
    val result =
      EditorViewportTransform(pageOffsets = offsets, displayZoom = 2f)
        .localToGlobal(page = 1, x = 100f, y = 50f)
    assertNotNull(result)
    assertEquals(200f, result.x)
    assertEquals(700f, result.y)
  }

  @Test
  fun globalToLocal_maps_viewport_coordinates_back_into_page_local_space() {
    val point =
      EditorViewportTransform(pageOffsets = offsetsCentered, pageSizes = sizesCentered)
        .globalToLocal(150f, 300f)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(100f, point.x)
    assertEquals(300f, point.y)
  }

  @Test
  fun globalToLocal_maps_display_coordinates_back_into_page_local_space_with_zoom() {
    val point =
      EditorViewportTransform(
          pageOffsets = offsetsCentered,
          pageSizes = sizesCentered,
          displayZoom = 2f,
        )
        .globalToLocal(x = 250f, y = 600f)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(100f, point.x)
    assertEquals(300f, point.y)
  }

  @Test
  fun zoom_anchor_must_be_resolved_from_the_pre_zoom_transform() {
    val focalX = 250f
    val focalY = 600f

    val pointBeforeZoom =
      EditorViewportTransform(
          pageOffsets = offsetsCentered,
          pageSizes = sizesCentered,
          displayZoom = 2f,
        )
        .globalToLocal(x = focalX, y = focalY)
    val pointAfterZoom =
      EditorViewportTransform(
          pageOffsets = offsetsCentered,
          pageSizes = sizesCentered,
          displayZoom = 1.5f,
        )
        .globalToLocal(x = focalX, y = focalY)

    assertNotNull(pointBeforeZoom)
    assertNotNull(pointAfterZoom)
    assertEquals(100f, pointBeforeZoom.x)
    assertEquals(300f, pointBeforeZoom.y)
    assertEquals(133.33333f, pointAfterZoom.x, 0.0001f)
    assertEquals(400f, pointAfterZoom.y)
  }

  @Test
  fun globalToLocal_clamps_coordinates_to_page_bounds() {
    val point =
      EditorViewportTransform(pageOffsets = offsets, pageSizes = sizes).globalToLocal(500f, 2000f)
    assertNotNull(point)
    assertEquals(2, point.page)
    assertEquals(400f, point.x)
    assertEquals(500f, point.y)
  }

  @Test
  fun globalToLocal_snaps_gap_touches_to_the_nearest_page_edge() {
    val point =
      EditorViewportTransform(pageOffsets = offsetsWithGap, pageSizes = sizes)
        .globalToLocal(100f, 610f)
    assertNotNull(point)
    assertEquals(1, point.page)
    assertEquals(0f, point.y)
  }

  @Test
  fun globalToLocal_can_snap_gap_touches_to_the_previous_page() {
    val point =
      EditorViewportTransform(pageOffsets = offsetsWithGap, pageSizes = sizes)
        .globalToLocal(100f, 605f)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(600f, point.y)
  }

  @Test
  fun globalToLocal_returns_null_without_page_metrics() {
    assertNull(EditorViewportTransform(pageOffsets = offsets).localToGlobal(5, 0f, 0f))
    assertNull(
      EditorViewportTransform(pageOffsets = emptyMap(), pageSizes = emptyList())
        .globalToLocal(0f, 0f)
    )
    assertNull(
      EditorViewportTransform(pageOffsets = emptyMap(), pageSizes = sizes).globalToLocal(0f, 0f)
    )
  }
}
