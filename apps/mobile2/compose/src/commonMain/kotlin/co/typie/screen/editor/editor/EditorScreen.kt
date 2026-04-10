package co.typie.screen.editor.editor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.runtime.CompositionLocalProvider
import co.typie.editor.compose.EditorView
import co.typie.editor.LocalEditorState
import co.typie.ui.component.Screen
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarTitle
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun EditorScreen(slug: String) {
  ProvideTopBar(
    center = { TopBarTitle("문서") },
  )

  val model = viewModel { EditorViewModel() }

  Screen(body = { contentPadding ->
    CompositionLocalProvider(LocalEditorState provides model.editorState) {
      Box(Modifier.fillMaxSize().padding(contentPadding)) {
        EditorView(doc = model.doc, selection = model.selection)
      }
    }
  })
}
