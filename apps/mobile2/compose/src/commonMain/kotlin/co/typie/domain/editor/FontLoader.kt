package co.typie.domain.editor

import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FontData
import co.typie.editor.ffi.FontFamily
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.generated.resources.Res
import co.typie.network.Http
import co.typie.platform.PlatformModule
import io.ktor.client.request.get
import io.ktor.client.statement.bodyAsBytes
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

private const val CDN_BASE = "https://cdn.typie.net/editor/fonts"
private const val PRELOAD_CONCURRENCY = 4

private data class FontPathEntry(val path: String, val hash: String)

@Serializable
private data class FallbackFamily(
  @SerialName("familyName") val familyName: String,
  @SerialName("fonts") val fonts: List<FallbackFont>,
)

@Serializable
private data class FallbackFont(
  @SerialName("weight") val weight: Int,
  @SerialName("path") val path: String,
  @SerialName("hash") val hash: String,
)

@Serializable private data class HashResponse(@SerialName("hash") val hash: String)

object FontLoader {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

  private val primaryFontPaths = mapOf("Pretendard" to "Pretendard-Regular")
  private val fontPaths = mutableMapOf<String, FontPathEntry>()

  private val loaded = mutableSetOf<String>()
  private val loading = mutableMapOf<String, CompletableDeferred<Unit>>()

  private val preloadQueue = PreloadQueue()

  private var initialized = false

  private fun fontKey(family: String, weight: Int): String = "$family:$weight"

  suspend fun initFonts() {
    if (initialized) return
    initialized = true

    val phantomFonts =
      listOf(
        "Noto (Phantom)" to "files/editor/Noto-Phantom.bin",
        "Noto Emoji (Phantom)" to "files/editor/Noto-Phantom-Emoji.bin",
      )

    for ((familyName, path) in phantomFonts) {
      val data = Res.readBytes(path)
      PlatformModule.editorHost.loadFontBase(familyName, 400, data)
    }

    PlatformModule.editorHost.setPhantomFontFamilies(phantomFonts.map { it.first })

    val fallbackManifestData = Res.readBytes("files/editor/fallbacks.bin")
    PlatformModule.editorHost.loadFallbackFontManifests(fallbackManifestData)

    val fallbackFamilies =
      Json.decodeFromString<List<FallbackFamily>>(
        Res.readBytes("files/editor/fallbacks.json").decodeToString()
      )

    for (family in fallbackFamilies) {
      for (font in family.fonts) {
        fontPaths[fontKey(family.familyName, font.weight)] = FontPathEntry(font.path, font.hash)
      }
    }

    PlatformModule.editorHost.setFontFamilies(listOf(FontFamily("Pretendard", listOf(400))))
  }

  val fontManifestMissingHandler: EditorEventListener<EditorEvent.FontManifestMissing> =
    { editor, event ->
      loadManifest(editor, event.family, event.weight)
    }

  val fontDataMissingHandler: EditorEventListener<EditorEvent.FontDataMissing> = { editor, event ->
    loadData(editor, event.family, event.weight, event.required, event.prefetch)
  }

  private fun loadManifest(editor: Editor, family: String, weight: Int) {
    val fontPath = primaryFontPaths[family] ?: return

    editor.scope.launch {
      loadOnce("manifest:$family:$weight") {
        coroutineScope {
          val manifestDeferred = async {
            Http.get("$CDN_BASE/$fontPath/manifest.bin").bodyAsBytes()
          }
          val hashDeferred = async {
            val data = Http.get("$CDN_BASE/$fontPath/hash.json").bodyAsBytes()
            Json.decodeFromString<HashResponse>(data.decodeToString()).hash
          }

          val manifest = manifestDeferred.await()
          val hash = hashDeferred.await()

          fontPaths[fontKey(family, weight)] = FontPathEntry(fontPath, hash)
          PlatformModule.editorHost.loadFontManifest(family, weight, manifest)
        }
      }

      editor.enqueue(Message.System(SystemEvent.FontManifestLoaded(family, weight)))
    }
  }

  private fun loadData(
    editor: Editor,
    family: String,
    weight: Int,
    required: List<FontData>,
    prefetch: List<FontData>,
  ) {
    val info = fontPaths[fontKey(family, weight)] ?: return
    val baseUrl = "$CDN_BASE/${info.path}/${info.hash}"

    editor.scope.launch {
      if (required.any { it is FontData.Base }) {
        loadOnce("base:$family:$weight") {
          val data = getOrFetch("$baseUrl/base.bin")
          PlatformModule.editorHost.loadFontBase(family, weight, data)
        }

        editor.enqueue(Message.System(SystemEvent.FontBaseLoaded(family, weight)))
      }

      val requiredChunks = required.filterIsInstance<FontData.Chunk>()

      coroutineScope {
        for (chunk in requiredChunks) {
          launch {
            loadOnce("chunk:$family:$weight:${chunk.index}") {
              val data = getOrFetch("$baseUrl/chunks/${chunk.index}.bin")
              PlatformModule.editorHost.loadFontChunk(family, weight, data)
            }

            editor.enqueue(Message.System(SystemEvent.FontChunkLoaded(family, weight)))
          }
        }
      }

      val prefetchChunks = prefetch.filterIsInstance<FontData.Chunk>()
      for (chunk in prefetchChunks) {
        preloadQueue.enqueue("chunk:$family:$weight:${chunk.index}", chunk.index) {
          loadOnce("chunk:$family:$weight:${chunk.index}") {
            val data = getOrFetch("$baseUrl/chunks/${chunk.index}.bin")
            PlatformModule.editorHost.loadFontChunk(family, weight, data)
          }

          editor.enqueue(Message.System(SystemEvent.FontChunkLoaded(family, weight)))
        }
      }
    }
  }

  private suspend fun loadOnce(key: String, block: suspend () -> Unit) {
    if (key in loaded) return

    val existing = loading[key]
    if (existing != null) {
      existing.await()
      return
    }

    val deferred = CompletableDeferred<Unit>()
    loading[key] = deferred
    try {
      block()
      loaded.add(key)
      deferred.complete(Unit)
    } catch (e: Exception) {
      deferred.completeExceptionally(e)
      throw e
    } finally {
      loading.remove(key)
    }
  }

  private suspend fun getOrFetch(url: String): ByteArray {
    PlatformModule.diskCache.get(url)?.let {
      return it
    }
    val data = Http.get(url).bodyAsBytes()
    PlatformModule.diskCache.put(url, data)
    return data
  }

  private class PreloadItem(val key: String, val priority: Int, val block: suspend () -> Unit)

  private class PreloadQueue {
    private val mutex = Mutex()
    private val pending = mutableListOf<PreloadItem>()
    private var inflight = 0

    suspend fun enqueue(key: String, priority: Int, block: suspend () -> Unit) {
      mutex.withLock {
        if (key in loaded) return
        val insertAt =
          pending.indexOfFirst { it.priority < priority }.let { if (it == -1) pending.size else it }
        pending.add(insertAt, PreloadItem(key, priority, block))
      }

      flush()
    }

    private suspend fun flush() {
      while (true) {
        val item =
          mutex.withLock {
            if (inflight >= PRELOAD_CONCURRENCY || pending.isEmpty()) return
            val item = pending.removeFirst()
            if (item.key in loaded) return@withLock null
            inflight++
            item
          } ?: continue

        scope.launch {
          try {
            item.block()
          } catch (_: Exception) {
            // best-effort
          } finally {
            withContext(NonCancellable) {
              mutex.withLock { inflight-- }
              flush()
            }
          }
        }
      }
    }
  }
}
