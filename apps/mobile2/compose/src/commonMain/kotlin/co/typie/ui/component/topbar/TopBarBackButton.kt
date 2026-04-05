package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.icons.Lucide
import co.typie.navigation.Nav

@Composable
fun TopBarBackButton(
  modifier: Modifier = Modifier,
  onClick: (suspend () -> Unit)? = null,
) {
  val navigator = Nav.current
  TopBarButton(
    icon = Lucide.ChevronLeft,
    onClick = onClick ?: { navigator.pop() },
    modifier = modifier,
  )
}
