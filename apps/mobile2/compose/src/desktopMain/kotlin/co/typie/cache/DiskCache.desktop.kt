package co.typie.cache

import okio.Path
import okio.Path.Companion.toPath

actual fun diskCacheDir(): Path = "${System.getProperty("user.home")}/.cache/typie/cache".toPath()
