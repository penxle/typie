package co.typie.ui.component.topbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.ui.icon.IconData

@Composable
fun TopBarBackButton(
  modifier: Modifier = Modifier,
  icon: IconData = Lucide.ChevronLeft,
  onClick: (suspend () -> Unit)? = null,
) {
  val navigator = Nav.current
  TopBarButton(
    icon = icon,
    onClick =
      onClick
        ?: {
          navigator.pop()
          Unit
        },
    modifier = modifier,
  )
}
