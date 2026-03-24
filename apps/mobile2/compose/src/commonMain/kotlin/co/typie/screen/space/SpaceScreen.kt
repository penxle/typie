package co.typie.screen.space

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme

@Composable
fun SpaceScreen() {
//  val model = koinViewModel<SpaceViewModel>()

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("스페이스", style = AppTheme.typography.title) },
  )

  Screen(
    modifier = Modifier.background(AppTheme.colors.surfaceBase),
  ) { _ ->
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
      Text("Space", style = AppTheme.typography.display)
    }
  }
}
