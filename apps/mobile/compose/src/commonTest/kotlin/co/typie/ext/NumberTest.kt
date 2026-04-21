package co.typie.ext

import kotlin.test.Test
import kotlin.test.assertEquals

class NumberTest {
  @Test
  fun comma_formats_int_with_grouping() {
    assertEquals("0", 0.comma)
    assertEquals("1,234", 1234.comma)
    assertEquals("1,234,567", 1_234_567.comma)
  }

  @Test
  fun comma_formats_negative_long_with_grouping() {
    assertEquals("-9,876,543,210", (-9_876_543_210L).comma)
  }
}
