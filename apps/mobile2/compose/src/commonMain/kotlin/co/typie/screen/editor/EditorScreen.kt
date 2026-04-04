package co.typie.screen.editor

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.Message
import co.typie.editor.SystemEvent
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
  var inspectedState by remember { mutableStateOf<String?>(null) }

  LaunchedEffect(Unit) {
    try {
      val editor = model.ensureEditor(scaleFactor = density.toDouble())
      editor.enqueue(Message.System(SystemEvent.Initialize))
      editor.tick()
      inspectedState = editor.inspectState(options = null)
    } catch (e: Exception) {
      inspectedState = e.message
    }
  }

  Screen { contentPadding ->
    Column(Modifier.fillMaxSize().padding(contentPadding)) {
      when (inspectedState) {
        null -> Text("Editor loading...", style = AppTheme.typography.body)
        else -> Text(
          "Editor: $slug\n$inspectedState",
          style = AppTheme.typography.body
        )
      }
    }
  }
}
