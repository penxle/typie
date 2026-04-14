package co.typie.screen.space.notes

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.shell.MainBottomBarActionButton
import co.typie.shell.MainBottomBarPill
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme

@Composable
fun NotesScreen() {
  //  val model = viewModel { NotesViewModel() }

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("노트", style = AppTheme.typography.title) },
  )

  ProvideBottomBar(pill = { MainBottomBarPill() }, action = { MainBottomBarActionButton() })

  Screen { _ ->
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
      Text("Notes", style = AppTheme.typography.display)
    }
  }
}
