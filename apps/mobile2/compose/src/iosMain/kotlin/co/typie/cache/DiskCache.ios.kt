package co.typie.cache

import co.typie.di.PlatformContext
import okio.Path
import okio.Path.Companion.toPath
import platform.Foundation.NSCachesDirectory
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSUserDomainMask

@Suppress("UNCHECKED_CAST")
actual fun diskCacheDir(ctx: PlatformContext): Path {
  val paths = NSSearchPathForDirectoriesInDomains(NSCachesDirectory, NSUserDomainMask, true) as List<String>
  return "${paths.first()}/cache".toPath()
}
