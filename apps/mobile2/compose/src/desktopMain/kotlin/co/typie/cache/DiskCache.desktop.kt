package co.typie.cache

import co.typie.di.PlatformContext
import okio.Path
import okio.Path.Companion.toPath

actual fun diskCacheDir(ctx: PlatformContext): Path =
  "${System.getProperty("user.home")}/.cache/typie/cache".toPath()
