package co.typie.cache

import co.typie.di.PlatformContext
import okio.Path
import okio.Path.Companion.toOkioPath
import java.io.File

actual fun diskCacheDir(ctx: PlatformContext): Path =
  File(ctx.context.cacheDir, "cache").toOkioPath()
