package co.typie.editor

import uniffi.editor.EditorEngine as NativeEditorEngine
import uniffi.editor.Editor as NativeEditor

class JnaEditorEngine(private val native: NativeEditorEngine) : EditorEngine {
    override fun createEditor(scaleFactor: Double, snapshot: ByteArray?): Editor {
        return JnaEditor(native.createEditor(scaleFactor, snapshot))
    }

    override fun close() {
        native.close()
    }
}

class JnaEditor(private val native: NativeEditor) : Editor {
    override fun dispatch(messageJson: String) {
        native.dispatch(messageJson)
    }

    override fun tick() {
        native.tick()
    }

    override fun exportSnapshot(): ByteArray {
        return native.exportSnapshot()
    }

    override fun close() {
        native.close()
    }
}
