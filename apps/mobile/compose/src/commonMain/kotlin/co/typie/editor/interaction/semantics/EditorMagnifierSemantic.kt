package co.typie.editor.interaction.semantics

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset

internal class EditorMagnifierSemantic {
  var position: Offset? by mutableStateOf(null)
    private set

  fun show(position: Offset) {
    this.position = position
  }

  fun hide() {
    position = null
  }

  fun reset() {
    hide()
  }
}
