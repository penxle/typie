package co.typie.editor.body

import androidx.compose.ui.geometry.Offset
import co.typie.editor.runtime.EditorBoundsInContainer
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorExtensionAreaPointerCoordinateResolverTest {
  @Test
  fun `start inside editor bounds is not forwarded by extension area`() {
    val resolver = resolver()

    assertNull(resolver.positionForStart(Offset(60f, 100f)))
    assertEquals(Offset(20f, 20f), resolver.positionForActivePointer(Offset(60f, 100f)))
  }

  @Test
  fun `start outside editor bounds is clamped to nearest editor edge`() {
    val resolver = resolver()

    assertEquals(Offset.Zero, resolver.positionForStart(Offset.Zero))
    assertEquals(Offset(320f, 480f), resolver.positionForStart(Offset(500f, 700f)))
  }

  @Test
  fun `invalid geometry does not forward extension area pointer positions`() {
    val invalidBounds = resolver(bounds = EditorBoundsInContainer())
    val invalidDensity = resolver(density = 0f)

    assertNull(invalidBounds.positionForStart(Offset.Zero))
    assertNull(invalidBounds.positionForActivePointer(Offset.Zero))
    assertNull(invalidDensity.positionForStart(Offset.Zero))
    assertNull(invalidDensity.positionForActivePointer(Offset.Zero))
  }

  private fun resolver(
    bounds: EditorBoundsInContainer =
      EditorBoundsInContainer(x = 20f, y = 40f, width = 160f, height = 240f),
    density: Float = 2f,
  ): EditorExtensionAreaPointerCoordinateResolver =
    EditorExtensionAreaPointerCoordinateResolver(bounds = bounds, density = density)
}
