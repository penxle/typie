package co.typie.cache

import co.typie.di.PlatformContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext
import okio.ByteString.Companion.encodeUtf8
import okio.FileSystem
import okio.Path
import okio.SYSTEM
import org.koin.core.annotation.Single

interface DiskCache {
  suspend fun get(url: String): ByteArray?
  suspend fun put(url: String, data: ByteArray)
}

@Single
fun diskCache(ctx: PlatformContext): DiskCache = OkioDiskCache(diskCacheDir(ctx))

expect fun diskCacheDir(ctx: PlatformContext): Path

private class OkioDiskCache(
  private val cacheDir: Path,
  private val fileSystem: FileSystem = FileSystem.SYSTEM,
) : DiskCache {

  override suspend fun get(url: String): ByteArray? = withContext(Dispatchers.IO) {
    val path = cacheDir / url.toFileName()
    if (!fileSystem.exists(path)) return@withContext null
    fileSystem.read(path) { readByteArray() }
  }

  override suspend fun put(url: String, data: ByteArray): Unit = withContext(Dispatchers.IO) {
    fileSystem.createDirectories(cacheDir)
    val path = cacheDir / url.toFileName()
    fileSystem.write(path) { write(data) }
    Unit
  }
}

private fun String.toFileName(): String = encodeUtf8().sha256().hex()
