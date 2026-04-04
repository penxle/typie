package co.typie.editor

import co.typie.di.PlatformContext
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import kotlinx.coroutines.runBlocking
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import uniffi.editor_ffi.EditorHost as NativeEditorHost

@Module
actual class EditorModule {
  @Single
  actual fun editorHost(ctx: PlatformContext): EditorHost {
    val native = runBlocking { NativeEditorHost.create(null) }

    val icu = JnaEditorHost::class.java.classLoader.getResourceAsStream("icu.zst")!!.readBytes()
    native.loadIcuData(icu)

    return JnaEditorHost(native)
  }
}
