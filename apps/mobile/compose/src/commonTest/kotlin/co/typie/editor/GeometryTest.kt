package co.typie.editor

import androidx.compose.ui.geometry.Offset
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
  fun localToGlobal_adds_page_offset() {
    val result = localToGlobal(1, 100f, 50f, offsets)
    assertNotNull(result)
    assertEquals(100f, result.x)
    assertEquals(650f, result.y)
  }

  @Test
  fun localToGlobal_scales_page_local_coordinates_with_display_zoom() {
    val result = localToGlobal(page = 1, x = 100f, y = 50f, pageOffsets = offsets, displayZoom = 2f)
    assertNotNull(result)
    assertEquals(200f, result.x)
    assertEquals(700f, result.y)
  }

  @Test
  fun globalToLocal_maps_viewport_coordinates_back_into_page_local_space() {
    val point = globalToLocal(150f, 300f, offsetsCentered, sizesCentered)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(100f, point.x)
    assertEquals(300f, point.y)
  }

  @Test
  fun globalToLocal_maps_display_coordinates_back_into_page_local_space_with_zoom() {
    val point =
      globalToLocal(
        x = 250f,
        y = 600f,
        pageOffsets = offsetsCentered,
        sizes = sizesCentered,
        displayZoom = 2f,
      )
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
      globalToLocal(
        x = focalX,
        y = focalY,
        pageOffsets = offsetsCentered,
        sizes = sizesCentered,
        displayZoom = 2f,
      )
    val pointAfterZoom =
      globalToLocal(
        x = focalX,
        y = focalY,
        pageOffsets = offsetsCentered,
        sizes = sizesCentered,
        displayZoom = 1.5f,
      )

    assertNotNull(pointBeforeZoom)
    assertNotNull(pointAfterZoom)
    assertEquals(100f, pointBeforeZoom.x)
    assertEquals(300f, pointBeforeZoom.y)
    assertEquals(133.33333f, pointAfterZoom.x, 0.0001f)
    assertEquals(400f, pointAfterZoom.y)
  }

  @Test
  fun globalToLocal_clamps_coordinates_to_page_bounds() {
    val point = globalToLocal(500f, 2000f, offsets, sizes)
    assertNotNull(point)
    assertEquals(2, point.page)
    assertEquals(400f, point.x)
    assertEquals(500f, point.y)
  }

  @Test
  fun globalToLocal_snaps_gap_touches_to_the_nearest_page_edge() {
    val point = globalToLocal(100f, 610f, offsetsWithGap, sizes)
    assertNotNull(point)
    assertEquals(1, point.page)
    assertEquals(0f, point.y)
  }

  @Test
  fun globalToLocal_can_snap_gap_touches_to_the_previous_page() {
    val point = globalToLocal(100f, 605f, offsetsWithGap, sizes)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(600f, point.y)
  }

  @Test
  fun globalToLocal_returns_null_without_page_metrics() {
    assertNull(localToGlobal(5, 0f, 0f, offsets))
    assertNull(globalToLocal(0f, 0f, emptyMap(), emptyList()))
    assertNull(globalToLocal(0f, 0f, emptyMap(), sizes))
  }
}
