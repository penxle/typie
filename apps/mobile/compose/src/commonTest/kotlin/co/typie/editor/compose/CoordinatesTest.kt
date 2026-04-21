package co.typie.editor.compose

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Size
import co.typie.editor.globalToLocal
import co.typie.editor.localToGlobal
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class CoordinatesTest {

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
  fun localToGlobal_returns_null_for_invalid_page() {
    assertNull(localToGlobal(5, 0f, 0f, offsets))
  }

  @Test
  fun globalToLocal_finds_correct_page() {
    val point = globalToLocal(100f, 650f, offsets, sizes)
    assertNotNull(point)
    assertEquals(1, point.page)
    assertEquals(100f, point.x)
    assertEquals(50f, point.y)
  }

  @Test
  fun globalToLocal_clamps_x_to_page_bounds() {
    val point = globalToLocal(500f, 300f, offsets, sizes)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(400f, point.x)
  }

  @Test
  fun globalToLocal_subtracts_page_x_offset() {
    val point = globalToLocal(150f, 300f, offsetsCentered, sizesCentered)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(100f, point.x)
  }

  @Test
  fun globalToLocal_clamps_x_relative_to_page_offset() {
    val point = globalToLocal(10f, 300f, offsetsCentered, sizesCentered)
    assertNotNull(point)
    assertEquals(0f, point.x)
  }

  @Test
  fun globalToLocal_with_gap_finds_correct_page() {
    val point = globalToLocal(100f, 700f, offsetsWithGap, sizes)
    assertNotNull(point)
    assertEquals(1, point.page)
    assertEquals(80f, point.y)
  }

  @Test
  fun globalToLocal_in_gap_snaps_to_nearest_page() {
    val point = globalToLocal(100f, 610f, offsetsWithGap, sizes)
    assertNotNull(point)
    assertEquals(1, point.page)
    assertEquals(0f, point.y)
  }

  @Test
  fun globalToLocal_in_gap_snaps_to_previous_page() {
    val point = globalToLocal(100f, 605f, offsetsWithGap, sizes)
    assertNotNull(point)
    assertEquals(0, point.page)
    assertEquals(600f, point.y)
  }

  @Test
  fun globalToLocal_returns_null_for_empty() {
    assertNull(globalToLocal(0f, 0f, emptyMap(), emptyList()))
  }

  @Test
  fun globalToLocal_returns_null_for_missing_offset() {
    assertNull(globalToLocal(0f, 0f, emptyMap(), sizes))
  }

  @Test
  fun globalToLocal_clamps_y_beyond_last_page() {
    val point = globalToLocal(100f, 2000f, offsets, sizes)
    assertNotNull(point)
    assertEquals(2, point.page)
    assertEquals(500f, point.y)
  }
}
