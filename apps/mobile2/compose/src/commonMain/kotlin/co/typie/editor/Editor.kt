package co.typie.editor

import co.typie.di.PlatformContext
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
expect class EditorModule() {
    @Single fun editorEngine(ctx: PlatformContext): EditorEngine
}

interface EditorEngine {
    fun createEditor(scaleFactor: Double, snapshot: ByteArray? = null): Editor
    fun close()
}

interface Editor {
    fun dispatch(messageJson: String)
    fun tick()
    fun exportSnapshot(): ByteArray
    fun close()
}
