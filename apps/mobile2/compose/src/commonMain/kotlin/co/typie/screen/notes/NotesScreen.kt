package co.typie.screen.notes

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun NotesScreen() {
  Screen { _ ->
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
      Text("Notes", style = AppTheme.typography.display)
    }
  }
}
