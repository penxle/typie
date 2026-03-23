package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.icons.Lucide
import co.typie.navigation.Nav

@Composable
fun TopBarBackButton(
  modifier: Modifier = Modifier,
) {
  val navigator = Nav.current
  TopBarButton(
    icon = Lucide.ChevronLeft,
    onClick = { navigator.pop() },
    modifier = modifier,
  )
}
