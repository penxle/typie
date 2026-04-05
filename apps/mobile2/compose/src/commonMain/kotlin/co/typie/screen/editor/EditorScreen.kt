package co.typie.screen.editor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.compose.EditorView
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarTitle
import co.typie.ui.theme.AppTheme
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun EditorScreen(slug: String) {
  ProvideTopBar(
    center = { TopBarTitle("문서") },
  )

  val model = koinViewModel<EditorViewModel>()
  val density = LocalDensity.current.density

  LaunchedEffect(Unit) {
    model.initialize(scaleFactor = density.toDouble())
  }

  val editor = model.editor

  Screen(body = { contentPadding ->
    Box(Modifier.fillMaxSize().padding(contentPadding)) {
      if (editor == null) {
        Text("Editor loading...", style = AppTheme.typography.body)
      } else {
        EditorView(editor)
      }
    }
  })
}
