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

private const val PRELOAD_CONCURRENCY = 4
private const val REQUIRED_LOAD_ATTEMPTS = 3
private const val PREFETCH_LOAD_ATTEMPTS = 1
private const val LOAD_RETRY_BASE_MS = 200L
internal const val RETRY_MAX_ATTEMPTS = 5
internal const val RETRY_BASE_MS = 2_000L
internal const val RETRY_CAP_MS = 30_000L

private data class FontPathEntry(val url: String, val hash: String)

internal data class RetryChain(val gen: Long, val attempt: Int)

private fun purgePrefixesOf(fontKeys: List<String>): List<String> = fontKeys.flatMap { fk ->
  listOf("manifest:$fk:", "base:$fk:", "chunk:$fk:")
}

internal class FontLoaderState {
  val loaded = mutableSetOf<String>()
  val loading = mutableMapOf<String, CompletableDeferred<Unit>>()
  val retryScheduled = mutableMapOf<String, RetryChain>()
  val generations = mutableMapOf<String, Long>()

  fun generationOf(fontKey: String): Long = generations[fontKey] ?: 0L

  fun isStale(fontKey: String, dispatchGen: Long): Boolean = generationOf(fontKey) != dispatchGen

  fun purge(fontKeys: List<String>, queuedKeys: MutableSet<String>) {
    for (fk in fontKeys) generations[fk] = generationOf(fk) + 1L
    val prefixes = purgePrefixesOf(fontKeys)
    loaded.removeAll { key -> prefixes.any { key.startsWith(it) } }
    loading.keys.removeAll { key -> prefixes.any { key.startsWith(it) } }
    retryScheduled.keys.removeAll { key -> prefixes.any { key.startsWith(it) } }
    queuedKeys.removeAll { key -> prefixes.any { key.startsWith(it) } }
  }
}

internal suspend fun FontLoaderState.loadOnce(
  mutex: Mutex,
  key: String,
  block: suspend () -> Boolean,
): Boolean {
  val deferred = CompletableDeferred<Unit>()
  val existing = mutex.withLock {
    // Invariant: `loaded` never outruns the Rust registry, or this early return would loop the
    // dispatch→commit→fan-out→re-resolve→re-emit cycle forever; holds because `loadFonts` purges
    // host state in the same critical section as the registry write.
    if (key in loaded) return true
    val inflight = loading[key]
    if (inflight == null) {
      loading[key] = deferred
    }
    inflight
  }
  if (existing != null) {
    existing.await()
    return mutex.withLock { key in loaded }
  }

  try {
    val committed = block()
    deferred.complete(Unit)
    return committed
  } catch (e: Exception) {
    deferred.completeExceptionally(e)
    throw e
  } finally {
    mutex.withLock {
      if (loading[key] === deferred) loading.remove(key)
    }
  }
}

internal suspend fun FontLoaderState.scheduleRetry(
  mutex: Mutex,
  scope: CoroutineScope,
  key: String,
  gen: Long,
  block: suspend () -> Unit,
) {
  val start = mutex.withLock {
    val existing = retryScheduled[key]
    if (existing != null && existing.gen == gen) return
    retryScheduled[key] = RetryChain(gen, 1)
    true
  }
  if (start) retryStep(mutex, scope, key, gen, 1, block)
}

internal suspend fun FontLoaderState.retryStep(
  mutex: Mutex,
  scope: CoroutineScope,
  key: String,
  gen: Long,
  attempt: Int,
  block: suspend () -> Unit,
) {
  if (attempt > RETRY_MAX_ATTEMPTS) {
    mutex.withLock { if (retryScheduled[key]?.gen == gen) retryScheduled.remove(key) }
    return
  }
  scope.launch {
    delay(minOf(RETRY_BASE_MS * (1L shl (attempt - 1)), RETRY_CAP_MS))
    val inflight = mutex.withLock {
      val chain = retryScheduled[key]
      if (chain == null || chain.gen != gen) return@launch
      if (key in loaded) {
        retryScheduled.remove(key)
        return@launch
      }
      retryScheduled[key] = RetryChain(gen, attempt)
      loading[key]
    }
    if (inflight != null) {
      runCatching { inflight.await() }
      val done = mutex.withLock { key in loaded }
      if (done) {
        mutex.withLock { if (retryScheduled[key]?.gen == gen) retryScheduled.remove(key) }
      } else {
        retryStep(mutex, scope, key, gen, attempt + 1, block)
      }
      return@launch
    }
    try {
      block()
      mutex.withLock { if (retryScheduled[key]?.gen == gen) retryScheduled.remove(key) }
    } catch (_: Exception) {
      retryStep(mutex, scope, key, gen, attempt + 1, block)
    }
  }
}

object FontLoader {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

  private val stateMutex = Mutex()

  @Volatile private var fontPaths: Map<String, FontPathEntry> = emptyMap()
  private val state = FontLoaderState()

  private val preloadQueue = PreloadQueue(stateMutex, state, scope)

  private fun fontKey(family: String, weight: Int): String = "$family:$weight"

  suspend fun loadFonts(families: List<FontLoader_FontFamily>) =
    withContext(Dispatchers.Default) {
      stateMutex.withLock {
        val next = buildMap {
          for (family in families) {
            for (font in family.fonts) {
              put(fontKey(family.familyName, font.weight), FontPathEntry(font.url, font.hash))
            }
          }
        }
        val changed =
          next.keys.filter { key -> fontPaths[key]?.hash != next[key]?.hash } +
            fontPaths.keys.filterNot { it in next }
        fontPaths = next
        purgeKeysLocked(changed)
        PlatformModule.editorHost.setFonts(families.map { it.toFfi() })
      }
      withContext(NonCancellable) {
        for (editor in EditorRegistry.snapshot()) {
          editor.enqueue(Message.System(SystemEvent.FontsChanged))
        }
      }
    }

  fun watchFonts(scope: CoroutineScope, families: () -> List<FontLoader_FontFamily>?) {
    scope.launch {
      snapshotFlow(families).filterNotNull().distinctUntilChanged().collect { loadFonts(it) }
    }
  }

  private fun purgeKeysLocked(fontKeys: List<String>) {
    if (fontKeys.isEmpty()) return
    state.purge(fontKeys, mutableSetOf())
    preloadQueue.purge(purgePrefixesOf(fontKeys))
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
    editor.scope.launch(Dispatchers.Default) {
      val fk = fontKey(family, weight)
      val (info, dispatchGen) = stateMutex.withLock { fontPaths[fk] to state.generationOf(fk) }
      if (info == null) return@launch
      val dispatchHash = info.hash
      val baseUrl = "${info.url}/${info.hash}"

      val hasManifest = required.any { it is FontData.Manifest }
      if (hasManifest) {
        try {
          load(
            family,
            weight,
            dispatchHash,
            dispatchGen,
            FontData.Manifest,
            baseUrl,
            REQUIRED_LOAD_ATTEMPTS,
          )
        } catch (_: Exception) {
          state.scheduleRetry(
            stateMutex,
            scope,
            keyOf(family, weight, dispatchHash, FontData.Manifest),
            dispatchGen,
          ) {
            load(
              family,
              weight,
              dispatchHash,
              dispatchGen,
              FontData.Manifest,
              baseUrl,
              REQUIRED_LOAD_ATTEMPTS,
            )
          }
        }
      }

      val hasBase = required.any { it is FontData.Base }
      val chunks = required.filterIsInstance<FontData.Chunk>()

      if (hasBase) {
        try {
          load(
            family,
            weight,
            dispatchHash,
            dispatchGen,
            FontData.Base,
            baseUrl,
            REQUIRED_LOAD_ATTEMPTS,
          )
        } catch (_: Exception) {
          state.scheduleRetry(
            stateMutex,
            scope,
            keyOf(family, weight, dispatchHash, FontData.Base),
            dispatchGen,
          ) {
            load(
              family,
              weight,
              dispatchHash,
              dispatchGen,
              FontData.Base,
              baseUrl,
              REQUIRED_LOAD_ATTEMPTS,
            )
          }
        }
      }

      coroutineScope {
        for (fd in chunks) {
          launch {
            try {
              load(
                family,
                weight,
                dispatchHash,
                dispatchGen,
                fd,
                baseUrl,
                REQUIRED_LOAD_ATTEMPTS,
              )
            } catch (_: Exception) {
              state.scheduleRetry(
                stateMutex,
                scope,
                keyOf(family, weight, dispatchHash, fd),
                dispatchGen,
              ) {
                load(
                  family,
                  weight,
                  dispatchHash,
                  dispatchGen,
                  fd,
                  baseUrl,
                  REQUIRED_LOAD_ATTEMPTS,
                )
              }
            }
          }
        }
      }

      for (fd in prefetch) {
        val priority =
          when (fd) {
            FontData.Manifest -> -2
            FontData.Base -> -1
            is FontData.Chunk -> fd.id
          }
        preloadQueue.enqueue(keyOf(family, weight, dispatchHash, fd), priority) {
          try {
            load(
              family,
              weight,
              dispatchHash,
              dispatchGen,
              fd,
              baseUrl,
              PREFETCH_LOAD_ATTEMPTS,
            )
          } catch (_: Exception) {
            // best-effort
          }
        }
      }
    }
  }

  private suspend fun load(
    family: String,
    weight: Int,
    dispatchHash: String,
    dispatchGen: Long,
    fd: FontData,
    baseUrl: String,
    attempts: Int,
  ) {
    val fk = fontKey(family, weight)
    val key = keyOf(family, weight, dispatchHash, fd)
    val committed =
      state.loadOnce(stateMutex, key) {
        var lastErr: Throwable? = null
        for (attempt in 1..attempts) {
          try {
            val url = urlOf(baseUrl, fd)
            val bytes = getOrFetch(url)
            val committed =
              try {
                stateMutex.withLock {
                  if (state.isStale(fk, dispatchGen)) {
                    false
                  } else {
                    when (fd) {
                      FontData.Manifest ->
                        PlatformModule.editorHost.addFontManifest(family, weight, bytes)
                      FontData.Base -> PlatformModule.editorHost.addFontBase(family, weight, bytes)
                      is FontData.Chunk ->
                        PlatformModule.editorHost.addFontChunk(family, weight, fd.id, bytes)
                    }
                    state.loaded.add(key)
                    true
                  }
                }
              } catch (e: Exception) {
                PlatformModule.diskCache.remove(url)
                throw e
              }
            return@loadOnce committed
          } catch (e: Exception) {
            lastErr = e
            if (attempt < attempts) {
              delay(LOAD_RETRY_BASE_MS * (1L shl (attempt - 1)))
            }
          }
        }
        throw lastErr ?: IllegalStateException("load failed without recorded error")
      }

    if (!committed) return
    val loadedEvent =
      when (fd) {
        FontData.Manifest -> SystemEvent.FontManifestLoaded(family, weight)
        FontData.Base -> SystemEvent.FontBaseLoaded(family, weight)
        is FontData.Chunk -> SystemEvent.FontChunkLoaded(family, weight, fd.id)
      }
    for (target in EditorRegistry.snapshot()) {
      target.enqueue(Message.System(loadedEvent))
    }
  }

  private fun keyOf(family: String, weight: Int, hash: String, fd: FontData): String =
    when (fd) {
      FontData.Manifest -> "manifest:$family:$weight:$hash"
      FontData.Base -> "base:$family:$weight:$hash"
      is FontData.Chunk -> "chunk:$family:$weight:$hash:${fd.id}"
    }

  private fun urlOf(baseUrl: String, fd: FontData): String =
    when (fd) {
      FontData.Manifest -> "$baseUrl/manifest.v1"
      FontData.Base -> "$baseUrl/base"
      is FontData.Chunk -> "$baseUrl/chunks/${fd.id}"
    }

  private suspend fun getOrFetch(url: String): ByteArray {
    PlatformModule.diskCache.get(url)?.let {
      return it
    }

    val data = Http.get(url).bodyAsBytes()
    PlatformModule.diskCache.put(url, data)

    return data
  }

  internal class PreloadQueue(
    private val mutex: Mutex,
    private val state: FontLoaderState,
    private val scope: CoroutineScope,
  ) {
    private val queued = mutableMapOf<String, PreloadItem>()
    private val pending = mutableListOf<PreloadItem>()
    private var inflight = 0

    suspend fun enqueue(key: String, priority: Int, block: suspend () -> Unit) {
      val item = PreloadItem(key, priority, block)
      mutex.withLock {
        if (key in state.loaded || key in queued) return
        queued[key] = item
        val insertAt =
          pending.indexOfFirst { it.priority < priority }.let { if (it == -1) pending.size else it }
        pending.add(insertAt, item)
      }
      flush()
    }

    private suspend fun flush() {
      while (true) {
        val item =
          mutex.withLock {
            if (inflight >= PRELOAD_CONCURRENCY || pending.isEmpty()) return
            val item = pending.removeAt(0)
            if (item.key in state.loaded) {
              if (queued[item.key] === item) queued.remove(item.key)
              return@withLock null
            }
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
              mutex.withLock {
                inflight--
                if (queued[item.key] === item) queued.remove(item.key)
              }
              flush()
            }
          }
        }
      }
    }

    fun purge(prefixes: List<String>) {
      pending.removeAll { item -> prefixes.any { item.key.startsWith(it) } }
      queued.keys.removeAll { key -> prefixes.any { key.startsWith(it) } }
    }

    private class PreloadItem(val key: String, val priority: Int, val block: suspend () -> Unit)
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
        )
      },
  )
