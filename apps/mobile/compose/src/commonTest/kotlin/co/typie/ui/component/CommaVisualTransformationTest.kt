package co.typie.ui.component

import androidx.compose.ui.text.AnnotatedString
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class CommaVisualTransformationTest {
  private val t = CommaVisualTransformation()

  @Test
  fun formatsThousands() {
    assertEquals("1,234,567", t.filter(AnnotatedString("1234567")).text.text)
    assertEquals("", t.filter(AnnotatedString("")).text.text)
  }

  @Test
  fun offsetMappingSkipsCommas() {
    val mapping = t.filter(AnnotatedString("1234567")).offsetMapping
    assertEquals(0, mapping.originalToTransformed(0))
    assertEquals(1, mapping.originalToTransformed(1))
    assertEquals(3, mapping.originalToTransformed(2))
    assertEquals(9, mapping.originalToTransformed(7))
    assertEquals(7, mapping.transformedToOriginal(9))
    assertEquals(2, mapping.transformedToOriginal(3))
  }

  @Test
  fun digitFilterCapsAt15() {
    assertEquals("123456789012345", "1234567890123456789".filterGoalDigits())
    assertEquals("1200", "1,200자".filterGoalDigits())
  }

  @Test
  fun digitFilterNormalizesLeadingZeros() {
    assertEquals("7", "007".filterGoalDigits())
    assertEquals("0", "000".filterGoalDigits())
    assertEquals("", "".filterGoalDigits())
    assertEquals("0", "0".filterGoalDigits())
    assertEquals("7", "000000000000007".filterGoalDigits())
    assertEquals("0", "000000000000000123".filterGoalDigits())
    assertEquals("7123456789012", "0071234567890123456".filterGoalDigits())
  }

  @Test
  fun goalTargetParsing() {
    assertEquals(50_000L, "50000".toGoalTargetOrNull())
    assertNull("".toGoalTargetOrNull())
    assertNull("0".toGoalTargetOrNull())
  }
}
