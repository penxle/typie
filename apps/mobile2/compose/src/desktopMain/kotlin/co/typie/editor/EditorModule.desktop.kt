package co.typie.editor

import co.typie.di.PlatformContext
import co.typie.editor.ffi.BackendKind
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import kotlinx.coroutines.runBlocking
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
actual class EditorModule {
  @Single
  actual fun editorHost(ctx: PlatformContext): EditorHost {
    val icuData = JnaEditorHost::class.java.classLoader.getResourceAsStream("icu.zst")!!.readBytes()
    return runBlocking { JnaEditorHost.create(BackendKind.Cpu, icuData) }
  }
}
