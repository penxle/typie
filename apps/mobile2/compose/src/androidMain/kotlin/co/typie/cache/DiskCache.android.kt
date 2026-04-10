package co.typie.cache

import co.typie.platform.PlatformModule
import okio.Path
import okio.Path.Companion.toOkioPath
import java.io.File

actual fun diskCacheDir(): Path =
  File(PlatformModule.context.cacheDir, "cache").toOkioPath()
