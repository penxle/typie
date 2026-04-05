package co.typie.editor

import co.typie.cache.DiskCache
import co.typie.di.PlatformContext
import co.typie.editor.ffi.EditorHost
import io.ktor.client.HttpClient
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
expect class EditorModule() {
  @Single fun editorHost(ctx: PlatformContext): EditorHost
}

@Single
fun fontLoader(host: EditorHost, httpClient: HttpClient, cache: DiskCache): FontLoader =
  FontLoader(host, httpClient, cache)

class EditorException(message: String) : RuntimeException(message)
