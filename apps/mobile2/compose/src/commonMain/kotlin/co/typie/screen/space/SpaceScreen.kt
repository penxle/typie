package co.typie.screen.space

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun SpaceScreen() {
  Screen { _ ->
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
      Text("Space", style = AppTheme.typography.display)
    }
  }
}
