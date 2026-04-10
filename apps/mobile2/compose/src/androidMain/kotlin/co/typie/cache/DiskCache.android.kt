package co.typie.cache

import co.typie.platform.PlatformModule
import java.io.File
import okio.Path
import okio.Path.Companion.toOkioPath

actual fun diskCacheDir(): Path = File(PlatformModule.context.cacheDir, "cache").toOkioPath()
