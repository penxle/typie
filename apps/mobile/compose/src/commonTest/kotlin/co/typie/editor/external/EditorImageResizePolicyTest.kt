package co.typie.editor.external

import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImageResizePolicyTest {
  @Test
  fun width_bounds_cap_max_at_original_image_width() {
    val bounds = imageResizeWidthBounds(boundsWidth = 800f, originalWidth = 320f)

    assertEquals(100f, bounds.min)
    assertEquals(320f, bounds.max)
  }

  @Test
  fun width_bounds_min_uses_larger_of_ten_percent_and_minimum_width() {
    val wide = imageResizeWidthBounds(boundsWidth = 1600f, originalWidth = 2000f)
    val narrow = imageResizeWidthBounds(boundsWidth = 600f, originalWidth = 2000f)

    assertEquals(160f, wide.min)
    assertEquals(100f, narrow.min)
  }

  @Test
  fun width_bounds_min_never_exceeds_max_width() {
    val bounds = imageResizeWidthBounds(boundsWidth = 800f, originalWidth = 64f)

    assertEquals(64f, bounds.min)
    assertEquals(64f, bounds.max)
  }

  @Test
  fun proportion_range_is_derived_from_width_bounds() {
    val range = imageResizeProportionRange(boundsWidth = 800f, originalWidth = 320f)

    assertEquals(13..40, range)
  }

  @Test
  fun proportion_for_width_rounds_to_nearest_percent() {
    assertEquals(38, imageResizeProportionForWidth(width = 300.8f, boundsWidth = 800f))
  }
}
