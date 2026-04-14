package co.typie.ui.theme

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.dp
import co.typie.ui.shape.SquircleShape
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class AppShapesTest {

  @Test
  fun radiusScaleValues() {
    assertEquals(6.dp, AppShapes.sm)
    assertEquals(12.dp, AppShapes.md)
    assertEquals(16.dp, AppShapes.lg)
    assertEquals(24.dp, AppShapes.xl)
    assertEquals(999.dp, AppShapes.full)
  }

  @Test
  fun circleIsCircleShape() {
    assertEquals(CircleShape, AppShapes.circle)
  }

  @Test
  fun roundedFactoryCreatesRoundedCornerShape() {
    assertIs<RoundedCornerShape>(AppShapes.rounded(AppShapes.md))
  }

  @Test
  fun squircleFactoryCreatesSquircleShape() {
    assertIs<SquircleShape>(AppShapes.squircle(AppShapes.lg))
  }
}
