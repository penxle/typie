package co.typie.editor

import co.typie.di.PlatformContext
import co.typie.editor.ffi.EditorHost
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
expect class EditorModule() {
    @Single fun editorHost(ctx: PlatformContext): EditorHost
}

class EditorException(message: String) : RuntimeException(message)
