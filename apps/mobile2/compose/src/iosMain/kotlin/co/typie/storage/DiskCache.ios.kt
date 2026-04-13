package co.typie.storage

import okio.Path
import okio.Path.Companion.toPath
import platform.Foundation.NSCachesDirectory
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSUserDomainMask

@Suppress("UNCHECKED_CAST")
actual fun diskCacheDir(): Path {
  val paths =
    NSSearchPathForDirectoriesInDomains(NSCachesDirectory, NSUserDomainMask, true) as List<String>
  return "${paths.first()}/cache".toPath()
}
