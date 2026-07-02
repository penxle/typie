package co.typie.editor.render

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class DamageUploadTest {
  @Test
  fun `damageRowRange with single rect returns its y-span`() {
    val damage = intArrayOf(3, 8, 6, 9)
    assertEquals(RowRange(8, 9), damageRowRange(damage, count = 1, height = 10))
  }

  @Test
  fun `damageRowRange with multiple rects unions the y-span`() {
    val damage = intArrayOf(0, 2, 4, 4, 1, 7, 3, 9)
    assertEquals(RowRange(2, 9), damageRowRange(damage, count = 2, height = 10))
  }

  @Test
  fun `damageRowRange clamps y0 below zero and y1 above height`() {
    val damage = intArrayOf(0, -2, 4, 15)
    assertEquals(RowRange(0, 10), damageRowRange(damage, count = 1, height = 10))
  }

  @Test
  fun `damageRowRange with zero count returns empty range`() {
    val damage = intArrayOf(0, 2, 4, 4)
    assertEquals(RowRange(0, 0), damageRowRange(damage, count = 0, height = 10))
  }

  @Test
  fun `damageRowRange fully outside height clamps to empty range`() {
    val damage = intArrayOf(0, 12, 4, 20)
    assertEquals(RowRange(0, 0), damageRowRange(damage, count = 1, height = 10))
  }

  private fun partial(
    hasCached: Boolean = true,
    cachedW: Int = 100,
    cachedH: Int = 200,
    w: Int = 100,
    h: Int = 200,
    readerLastVersion: Long = 5,
    damageFrom: Long = 3,
    damageCount: Int = 2,
  ) =
    shouldPartialUpload(
      hasCached,
      cachedW,
      cachedH,
      w,
      h,
      readerLastVersion,
      damageFrom,
      damageCount,
    )

  @Test
  fun `shouldPartialUpload true when cached dims match and version covers damage`() {
    assertTrue(partial())
  }

  @Test
  fun `shouldPartialUpload false when reader lags behind damage start`() {
    assertFalse(partial(readerLastVersion = 2, damageFrom = 3))
  }

  @Test
  fun `shouldPartialUpload false when cached width differs from current width`() {
    assertFalse(partial(cachedW = 99))
  }

  @Test
  fun `shouldPartialUpload false on first frame with no reader version`() {
    assertFalse(partial(readerLastVersion = 0))
  }

  @Test
  fun `shouldPartialUpload false when damage count is zero`() {
    assertFalse(partial(damageCount = 0))
  }

  @Test
  fun `shouldPartialUpload false when there is no cached buffer`() {
    assertFalse(partial(hasCached = false))
  }
}
