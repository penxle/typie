package co.typie.editor.body

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.EditorUiState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorExtensionAreaPointerCoordinateResolverTest {
  @Test
  fun `single interaction boundary owns starts inside editor bounds`() {
    val resolver = resolver()

    assertEquals(Offset(20f, 20f), resolver.positionForTapStart(Offset(60f, 100f)))
    assertEquals(Offset(20f, 20f), resolver.positionForPointerStart(Offset(60f, 100f)))
    assertEquals(Offset(20f, 20f), resolver.positionForActivePointer(Offset(60f, 100f)))
    assertEquals(Offset(160f, 300f), resolver.positionInRoot(Offset(60f, 100f)))
  }

  @Test
  fun `continuous start outside editor bounds is clamped as a tap start`() {
    val resolver = resolver()

    assertEquals(Offset.Zero, resolver.positionForTapStart(Offset.Zero))
    assertEquals(Offset.Zero, resolver.positionForPointerStart(Offset.Zero))
    assertEquals(Offset(320f, 480f), resolver.positionForTapStart(Offset(500f, 700f)))
    assertEquals(Offset(320f, 480f), resolver.positionForPointerStart(Offset(500f, 700f)))
    assertEquals(Offset(600f, 900f), resolver.positionInRoot(Offset(500f, 700f)))
  }

  @Test
  fun `paginated start outside editor bounds is tracked but not accepted as a tap start`() {
    val resolver = resolver(layoutSpec = paginatedLayoutSpec)

    assertEquals(Offset.Zero, resolver.positionForPointerStart(Offset.Zero))
    assertNull(resolver.positionForTapStart(Offset.Zero))
  }

  @Test
  fun `invalid geometry does not forward extension area pointer positions`() {
    val invalidBounds = resolver(bounds = EditorBoundsInContainer())
    val invalidDensity = resolver(density = 0f)

    assertNull(invalidBounds.positionForPointerStart(Offset.Zero))
    assertNull(invalidBounds.positionForTapStart(Offset.Zero))
    assertNull(invalidBounds.positionForActivePointer(Offset.Zero))
    assertNull(invalidDensity.positionForPointerStart(Offset.Zero))
    assertNull(invalidDensity.positionForTapStart(Offset.Zero))
    assertNull(invalidDensity.positionForActivePointer(Offset.Zero))
  }

  private fun resolver(
    layoutSpec: EditorDocumentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 160f),
    bounds: EditorBoundsInContainer =
      EditorBoundsInContainer(x = 20f, y = 40f, width = 160f, height = 240f),
    density: Float = 2f,
  ): EditorExtensionAreaPointerCoordinateResolver =
    EditorExtensionAreaPointerCoordinateResolver(
      layoutSpec = layoutSpec,
      uiState =
        EditorUiState().apply {
          updateExtensionAreaBounds(
            boundsInRoot = Rect(left = 100f, top = 200f, right = 1100f, bottom = 1600f),
            density = density,
          )
          if (bounds.isValid) {
            updateEditorBounds(
              boundsInRoot =
                Rect(
                  left = 100f + bounds.x * density,
                  top = 200f + bounds.y * density,
                  right = 100f + (bounds.x + bounds.width) * density,
                  bottom = 200f + (bounds.y + bounds.height) * density,
                ),
              density = density,
            )
          }
        },
      density = density,
    )

  private val paginatedLayoutSpec =
    EditorDocumentLayoutSpec.Paginated(
      pageWidth = 320f,
      pageHeight = 480f,
      pageMarginTop = 48f,
      pageMarginBottom = 48f,
      pageMarginLeft = 40f,
      pageMarginRight = 40f,
    )
}
