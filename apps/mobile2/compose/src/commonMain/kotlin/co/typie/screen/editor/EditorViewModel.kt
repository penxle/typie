package co.typie.screen.editor

import androidx.lifecycle.ViewModel
import co.typie.editor.Editor
import co.typie.editor.EditorEngine
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class EditorViewModel(
    private val engine: EditorEngine,
) : ViewModel() {
    var editor: Editor? = null
        private set

    fun ensureEditor(scaleFactor: Double): Editor {
        return editor ?: engine.createEditor(scaleFactor = scaleFactor).also { editor = it }
    }

    override fun onCleared() {
        editor?.close()
    }
}
