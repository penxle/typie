package co.typie.editor

import androidx.compose.runtime.snapshotFlow
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FontData
import co.typie.editor.ffi.FontFamily
import co.typie.editor.ffi.FontFamilySource
import co.typie.editor.ffi.FontWeight
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.graphql.fragment.FontLoader_FontFamily
import co.typie.graphql.type.FontFamilySource as GraphqlFontFamilySource
import co.typie.network.Http
import co.typie.platform.PlatformModule
import co.typie.serialization.json
import io.ktor.client.request.get
import io.ktor.client.statement.bodyAsBytes
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.decodeFromJsonElement

private const val PRELOAD_CONCURRENCY = 4
private const val REQUIRED_LOAD_ATTEMPTS = 3
private const val PREFETCH_LOAD_ATTEMPTS = 1
private const val LOAD_RETRY_BASE_MS = 200L

private data class FontPathEntry(val url: String, val hash: String)

object FontLoader {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

  private val fontPaths = mutableMapOf<String, FontPathEntry>()

  private val loaded = mutableSetOf<String>()
  private val loading = mutableMapOf<String, CompletableDeferred<Unit>>()

  private val preloadQueue = PreloadQueue()

  private fun fontKey(family: String, weight: Int): String = "$family:$weight"

  suspend fun loadFonts(families: List<FontLoader_FontFamily>) =
    withContext(Dispatchers.Default) {
      updateFontPaths(families)
      PlatformModule.editorHost.setFonts(families.map { it.toFfi() })
      for (editor in EditorRegistry.snapshot()) {
        editor.enqueue(Message.System(SystemEvent.FontsChanged))
      }
    }

  fun watchFonts(scope: CoroutineScope, families: () -> List<FontLoader_FontFamily>?) {
    scope.launch {
      snapshotFlow(families).filterNotNull().distinctUntilChanged().collect { loadFonts(it) }
    }
  }

  private fun updateFontPaths(families: List<FontLoader_FontFamily>) {
    for (family in families) {
      for (font in family.fonts) {
        fontPaths[fontKey(family.familyName, font.weight)] = FontPathEntry(font.url, font.hash)
      }
    }
  }

  val fontDataMissingHandler: EditorEventListener<EditorEvent.FontDataMissing> = { editor, event ->
    loadFontData(editor, event.family, event.weight, event.required, event.prefetch)
  }

  private fun loadFontData(
    editor: Editor,
    family: String,
    weight: Int,
    required: List<FontData>,
    prefetch: List<FontData>,
  ) {
    val info = fontPaths[fontKey(family, weight)] ?: return
    val baseUrl = "${info.url}/${info.hash}"

    editor.scope.launch(Dispatchers.Default) {
      val hasBase = required.any { it is FontData.Base }
      val chunks = required.filterIsInstance<FontData.Chunk>()

      if (hasBase) {
        try {
          load(editor, family, weight, FontData.Base, baseUrl, REQUIRED_LOAD_ATTEMPTS)
        } catch (_: Exception) {
          // best-effort
        }
      }

      coroutineScope {
        for (fd in chunks) {
          launch {
            try {
              load(editor, family, weight, fd, baseUrl, REQUIRED_LOAD_ATTEMPTS)
            } catch (_: Exception) {
              // best-effort
            }
          }
        }
      }

      for (fd in prefetch) {
        val priority =
          when (fd) {
            FontData.Base -> -1
            is FontData.Chunk -> fd.id
          }
        preloadQueue.enqueue(keyOf(family, weight, fd), priority) {
          try {
            load(editor, family, weight, fd, baseUrl, PREFETCH_LOAD_ATTEMPTS)
          } catch (_: Exception) {
            // best-effort
          }
        }
      }
    }
  }

  private suspend fun load(
    editor: Editor,
    family: String,
    weight: Int,
    fd: FontData,
    baseUrl: String,
    attempts: Int,
  ) {
    loadOnce(keyOf(family, weight, fd)) {
      var lastErr: Throwable? = null
      for (attempt in 1..attempts) {
        try {
          val bytes = getOrFetch(urlOf(baseUrl, fd))
          when (fd) {
            FontData.Base -> PlatformModule.editorHost.addFontBase(family, weight, bytes)
            is FontData.Chunk ->
              PlatformModule.editorHost.addFontChunk(family, weight, fd.id, bytes)
          }
          return@loadOnce
        } catch (e: Exception) {
          lastErr = e
          if (attempt < attempts) {
            delay(LOAD_RETRY_BASE_MS * (1L shl (attempt - 1)))
          }
        }
      }
      throw lastErr ?: IllegalStateException("load failed without recorded error")
    }

    val loadedEvent =
      when (fd) {
        FontData.Base -> SystemEvent.FontBaseLoaded(family, weight)
        is FontData.Chunk -> SystemEvent.FontChunkLoaded(family, weight, fd.id)
      }
    editor.enqueue(Message.System(loadedEvent))
  }

  private fun keyOf(family: String, weight: Int, fd: FontData): String =
    when (fd) {
      FontData.Base -> "base:$family:$weight"
      is FontData.Chunk -> "chunk:$family:$weight:${fd.id}"
    }

  private fun urlOf(baseUrl: String, fd: FontData): String =
    when (fd) {
      FontData.Base -> "$baseUrl/base"
      is FontData.Chunk -> "$baseUrl/chunks/${fd.id}"
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
            val item = pending.removeAt(0)
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

private fun FontLoader_FontFamily.toFfi(): FontFamily =
  FontFamily(
    name = familyName,
    source =
      when (source) {
        GraphqlFontFamilySource.DEFAULT -> FontFamilySource.Default
        GraphqlFontFamilySource.USER -> FontFamilySource.User
        GraphqlFontFamilySource.FALLBACK -> FontFamilySource.Fallback
        GraphqlFontFamilySource.UNKNOWN__ -> error("Unknown FontFamilySource from server: $source")
      },
    weights =
      fonts.map { f ->
        FontWeight(
          value = f.weight,
          hash = f.hash,
          chunks = json.decodeFromJsonElement<List<List<Int>>>(f.chunks),
        )
      },
  )
