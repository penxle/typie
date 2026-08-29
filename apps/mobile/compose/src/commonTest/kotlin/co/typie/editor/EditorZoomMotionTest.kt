package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EditorZoomMotionTest {
  private val bounds = 0.2f..2f

  @Test
  fun `rubber band overzoom keeps moving without a nearby second wall`() {
    assertEquals(1.5f, elasticEditorDisplayZoom(1.5f, bounds))
    assertEquals(2f, elasticEditorDisplayZoom(2f, bounds))

    val first = requireNotNull(elasticEditorDisplayZoom(2.1f, bounds))
    val second = requireNotNull(elasticEditorDisplayZoom(2.2f, bounds))
    assertTrue(first > 2f)
    assertTrue(first < 2.1f)
    assertTrue(second > first)
    assertTrue(second < 2.2f)
    assertTrue(
      requireNotNull(elasticEditorDisplayZoom(100f, bounds)) <
        bounds.endInclusive * EditorZoomMotionTuning.ElasticExtentRatio
    )
  }

  @Test
  fun `direct overzoom recovers to the normal bound`() {
    val motion = createMotion(displayZoom = 2.08f)

    assertTrue(motion.advance(1.0 / 60.0).displayZoom < 2.08f)
    assertEquals(2f, motion.advance(1.0).displayZoom, 0.0001f)
  }

  @Test
  fun `direct underzoom recovers to the normal bound`() {
    val motion = createMotion(displayZoom = 0.18f)

    assertTrue(motion.advance(1.0 / 60.0).displayZoom > 0.18f)
    assertEquals(0.2f, motion.advance(1.0).displayZoom, 0.0001f)
  }

  private fun createMotion(displayZoom: Float): EditorZoomMotion =
    EditorZoomMotion(displayZoom = displayZoom, bounds = bounds)
}
