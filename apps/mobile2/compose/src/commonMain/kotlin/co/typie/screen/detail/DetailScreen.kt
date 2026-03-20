package co.typie.screen.detail

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.navigation.Nav
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun DetailScreen(id: String) {
  val nav = Nav.current
  Screen { _ ->
    Column(Modifier.fillMaxSize().padding(16.dp)) {
      Text(
        "< Back",
        modifier = Modifier.clickable { nav.pop() }.padding(bottom = 16.dp),
      )
      Text("Detail: $id", style = AppTheme.typography.display)
    }
  }
}
