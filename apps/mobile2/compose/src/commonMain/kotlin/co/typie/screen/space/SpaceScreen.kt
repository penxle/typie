package co.typie.screen.space

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.sp
import co.typie.ui.component.Screen
import co.typie.ui.component.Text

@Composable
fun SpaceScreen() {
  Screen {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
      Text("Space", style = TextStyle(fontSize = 20.sp))
    }
  }
}
