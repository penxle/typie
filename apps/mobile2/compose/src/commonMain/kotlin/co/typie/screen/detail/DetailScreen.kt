package co.typie.screen.detail

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.icons.Lucide
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.TopBar
import co.typie.ui.component.topbar.TopBarTitle
import co.typie.ui.theme.AppTheme

@Composable
fun DetailScreen(id: String) {
  Screen(
    topBar = {
      TopBar(
        center = {
          TopBarTitle(id, subtitle = "Detail Screen", icon = Lucide.FolderOpen)
        }
      )
    }
  ) { contentPadding ->
    Column(Modifier.fillMaxSize().padding(contentPadding)) {
      Text("Detail: $id", style = AppTheme.typography.body)
    }
  }
}
