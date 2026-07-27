package co.typie.screen.editor.editor

import co.typie.editor.external.EditorAssetPendingMeta
import co.typie.editor.external.EditorAssetResolution
import co.typie.editor.external.EditorEmbedAsset
import co.typie.editor.external.EditorExternalAsset
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.sync.ws.WsAssetStateEntry
import co.typie.editor.sync.ws.WsReadyAsset
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal class EditorAssetHydrator(
  private val scope: CoroutineScope,
  private val state: EditorExternalElementState,
  private val pull: (requestId: String, ids: List<String>) -> Unit,
  private val basePollMs: Long = 60_000,
  private val maxPollMs: Long = 300_000,
  private val debounceMs: Long = 100,
  private val maxIdsPerPull: Int = MaxIdsPerPull,
) {
  private val instanceId = "s${++instanceSeq}"
  private var requestSeq = 0

  private var referenced = emptySet<String>()
  private val latestRequestIds = mutableMapOf<String, String>()
  private val awaiting = mutableSetOf<String>()
  private val queued = mutableSetOf<String>()

  private var debounceJob: Job? = null
  private var pollJob: Job? = null
  private var pollDelayMs = basePollMs
  private var disposed = false

  fun update(referencedIds: Collection<String>) {
    if (disposed) return
    setReferenced(referencedIds)
    enqueue(
      referenced.filter { id ->
        !state.containsAsset(id) && state.resolutions[id] == null && id !in awaiting
      }
    )
    ensurePoll()
  }

  fun invalidate(ids: Collection<String>) {
    if (disposed) return
    enqueue(ids)
    ensurePoll()
  }

  fun receive(requestId: String, entries: List<WsAssetStateEntry>, final: Boolean) {
    if (disposed) return

    var appliedAny = false
    for (entry in entries) {
      if (latestRequestIds[entry.id] != requestId || entry.id !in referenced) continue
      awaiting.remove(entry.id)

      val previous = currentDiscriminant(entry.id)
      if (previous == entry.state || (previous == "ready" && entry.state != "ready")) continue

      val applied =
        when (entry.state) {
          "ready" -> {
            val asset = entry.asset?.toEditorExternalAsset()
            if (asset == null) {
              false
            } else {
              state.put(asset)
              state.resolutions.remove(entry.id)
              true
            }
          }
          "pending" -> {
            val meta = entry.meta
            if (meta == null) {
              false
            } else {
              state.resolutions[entry.id] =
                EditorAssetResolution.Pending(
                  EditorAssetPendingMeta(kind = meta.kind, name = meta.name, size = meta.size)
                )
              true
            }
          }
          "missing" -> {
            state.resolutions[entry.id] = EditorAssetResolution.Missing
            true
          }
          else -> false
        }
      if (applied) appliedAny = true
    }

    if (final) {
      for ((id, issued) in latestRequestIds) {
        if (issued == requestId) awaiting.remove(id)
      }
    }

    if (appliedAny) {
      resetPoll()
    } else {
      ensurePoll()
    }
  }

  fun repullReferenced(referencedIds: Collection<String>) {
    if (disposed) return
    setReferenced(referencedIds)
    clearDebounce()
    queued.clear()
    val ids = nonReadyIds()
    if (ids.isNotEmpty()) dispatch(ids)
    resetPoll()
  }

  fun dispose() {
    disposed = true
    clearDebounce()
    clearPoll()
    referenced = emptySet()
    latestRequestIds.clear()
    awaiting.clear()
    queued.clear()
  }

  private fun currentDiscriminant(id: String): String? =
    when {
      state.containsAsset(id) -> "ready"
      state.resolutions[id] is EditorAssetResolution.Pending -> "pending"
      state.resolutions[id] == EditorAssetResolution.Missing -> "missing"
      else -> null
    }

  private fun nonReadyIds(): List<String> = referenced.filterNot(state::containsAsset)

  private fun dispatch(ids: List<String>) {
    var index = 0
    while (index < ids.size) {
      val chunk = ids.subList(index, minOf(index + maxIdsPerPull, ids.size))
      val requestId = "$instanceId-${++requestSeq}"
      for (id in chunk) {
        latestRequestIds[id] = requestId
        awaiting.add(id)
      }
      pull(requestId, chunk)
      index += maxIdsPerPull
    }
  }

  private fun clearDebounce() {
    debounceJob?.cancel()
    debounceJob = null
  }

  private fun flush() {
    debounceJob = null
    if (disposed) return
    val ids = queued.filter(referenced::contains)
    queued.clear()
    if (ids.isNotEmpty()) dispatch(ids)
  }

  private fun enqueue(ids: Collection<String>) {
    queued.addAll(ids)
    if (queued.isEmpty() || debounceJob != null) return
    debounceJob = scope.launch {
      delay(debounceMs)
      flush()
    }
  }

  private fun clearPoll() {
    pollJob?.cancel()
    pollJob = null
  }

  private fun ensurePoll() {
    if (disposed || pollJob != null || nonReadyIds().isEmpty()) return
    pollJob = scope.launch {
      delay(pollDelayMs)
      pollJob = null
      val ids = nonReadyIds()
      if (ids.isEmpty()) {
        pollDelayMs = basePollMs
        return@launch
      }
      dispatch(ids)
      pollDelayMs = minOf(pollDelayMs * 2, maxPollMs)
      ensurePoll()
    }
  }

  private fun resetPoll() {
    pollDelayMs = basePollMs
    clearPoll()
    ensurePoll()
  }

  private fun retire(id: String) {
    state.resolutions.remove(id)
    latestRequestIds.remove(id)
    awaiting.remove(id)
  }

  private fun setReferenced(referencedIds: Collection<String>) {
    val next = referencedIds.toSet()
    for (id in referenced) {
      if (id !in next) retire(id)
    }
    referenced = next
  }

  private companion object {
    var instanceSeq = 0
    const val MaxIdsPerPull = 100
  }
}

private fun WsReadyAsset.toEditorExternalAsset(): EditorExternalAsset? =
  when (type) {
    "image" -> {
      val w = (width ?: 0L).toInt()
      val h = (height ?: 0L).toInt()
      EditorImageAsset(
        id = id,
        url = url.orEmpty(),
        width = w,
        height = h,
        ratio = if (h > 0) w.toDouble() / h.toDouble() else 0.0,
        placeholder = placeholder,
      )
    }
    "file" -> EditorFileAsset(id = id, name = name.orEmpty(), url = url.orEmpty(), size = size)
    "embed" ->
      EditorEmbedAsset(
        id = id,
        url = url.orEmpty(),
        title = title,
        description = description,
        thumbnailUrl = thumbnailUrl,
        html = html,
      )
    // archived는 이 클라이언트가 원본 콘텐츠를 렌더하지 않으므로 참조 집합에도 넣지 않는다.
    else -> null
  }
