package co.typie.route

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class RouteTest {
  @Test
  fun `delete user toast bottom inset accounts for stacked bottom buttons`() {
    assertEquals(120.dp, Route.DeleteUser.toastBottomInset)
  }
}
