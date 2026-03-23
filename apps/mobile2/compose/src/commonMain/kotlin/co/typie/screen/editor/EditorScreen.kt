package co.typie.screen.editor

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarTitle
import co.typie.ui.theme.AppTheme

@Composable
fun EditorScreen(slug: String) {
  ProvideTopBar(
    center = { TopBarTitle("문서") },
  )

  Screen { contentPadding ->
    Column(Modifier.fillMaxSize().padding(contentPadding)) {
      Text("Editor: $slug", style = AppTheme.typography.body)
    }
  }
}
