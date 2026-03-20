package co.typie.screen.detail

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.navigation.Nav
import co.typie.ui.clickable
import co.typie.ui.component.Screen
import co.typie.ui.component.Text

@Composable
fun DetailScreen(id: String) {
  val nav = Nav.current
  Screen {
    Column(Modifier.fillMaxSize().padding(16.dp)) {
      Text(
        "< Back",
        style = TextStyle(fontSize = 16.sp),
        modifier = Modifier.clickable { nav.pop() }.padding(bottom = 16.dp),
      )
      Text("Detail: $id", style = TextStyle(fontSize = 20.sp))
    }
  }
}
