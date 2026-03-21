package co.typie.ui.component.topbar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.icons.Lucide
import co.typie.navigation.Nav

@Composable
fun TopBarBackButton(
  modifier: Modifier = Modifier,
) {
  val navigator = Nav.current
  AnimatedVisibility(
    visible = navigator.canPop,
    enter = fadeIn() + slideInHorizontally { -it },
    exit = fadeOut() + slideOutHorizontally { -it },
  ) {
    TopBarButton(
      icon = Lucide.ChevronLeft,
      onClick = { navigator.requestPop() },
      modifier = modifier,
    )
  }
}
