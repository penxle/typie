package co.typie.ui.shape

import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertIs

class SquircleShapeTest {

  private val density = Density(density = 1f, fontScale = 1f)

  @Test
  fun createsGenericOutline() {
    val shape = SquircleShape(12.dp)
    val outline = shape.createOutline(
      size = Size(100f, 50f),
      layoutDirection = LayoutDirection.Ltr,
      density = density,
    )
    assertIs<Outline.Generic>(outline)
  }

  @Test
  fun pillShapeWithLargeRadius() {
    val shape = SquircleShape(999.dp)
    val outline = shape.createOutline(
      size = Size(200f, 44f),
      layoutDirection = LayoutDirection.Ltr,
      density = density,
    )
    assertIs<Outline.Generic>(outline)
  }

  @Test
  fun zeroRadiusCreatesOutline() {
    val shape = SquircleShape(0.dp)
    val outline = shape.createOutline(
      size = Size(100f, 50f),
      layoutDirection = LayoutDirection.Ltr,
      density = density,
    )
    assertIs<Outline.Generic>(outline)
  }
}
