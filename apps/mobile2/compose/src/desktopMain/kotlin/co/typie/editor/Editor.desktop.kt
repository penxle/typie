package co.typie.editor

import co.typie.di.PlatformContext
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import uniffi.editor.EditorEngine as NativeEditorEngine

@Module
actual class EditorModule {
    @Single
    actual fun editorEngine(ctx: PlatformContext): EditorEngine {
        val icu = JnaEditorEngine::class.java.classLoader.getResourceAsStream("icu.zst")!!.readBytes()
        return JnaEditorEngine(NativeEditorEngine().also { it.loadIcuData(icu) })
    }
}
