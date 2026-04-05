package co.typie.editor

import co.typie.di.PlatformContext
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import kotlinx.coroutines.runBlocking
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
actual class EditorModule {
  @Single
  actual fun editorHost(ctx: PlatformContext): EditorHost {
    val host = runBlocking { JnaEditorHost.create() }

    val icu = ctx.context.assets.open("icu.zst").readBytes()
    host.loadIcuData(icu)

    return host
  }
}
