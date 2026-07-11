package co.typie.screen.editor.editor

import co.touchlab.kermit.Logger
import co.typie.editor.external.EditorAssetResolution
import co.typie.editor.external.EditorExternalAsset
import co.typie.editor.external.EditorExternalElementState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal class EditorAssetHydrator(
  private val state: EditorExternalElementState,
  private val fetch: suspend (List<String>) -> List<EditorExternalAsset>,
  private val waitBeforeRetry: suspend (attempt: Int) -> Unit = { attempt ->
    delay(if (attempt == 1) 500 else 1_500)
  },
) {
  private data class Request(
    val ids: List<String>,
    val queryGeneration: Long,
    val connectivityGeneration: Long,
    val recoveryIds: Set<String>,
  )

  private data class Retry(val attempts: Map<String, Int>)

  private val stateMutex = Mutex()
  private val resolverMutex = Mutex()
  private var referencedIds = emptySet<String>()
  private var queryGeneration = Long.MIN_VALUE
  private var lastConnectivityGeneration = Long.MIN_VALUE
  private val materializationAttempts = mutableMapOf<String, Int>()
  private val recoveryProbeIds = mutableSetOf<String>()

  suspend fun seed(assets: Collection<EditorExternalAsset>) {
    stateMutex.withLock { mergeAssets(assets) }
  }

  suspend fun resolve(ids: Collection<String>) {
    updateReferences(ids)
    drain()
  }

  suspend fun onQueryRefresh(generation: Long, assets: Collection<EditorExternalAsset>) {
    val startsNewGeneration = stateMutex.withLock {
      mergeAssets(assets)
      if (generation <= queryGeneration) {
        false
      } else {
        queryGeneration = generation
        for (id in referencedIds) {
          materializationAttempts.remove(id)
          recoveryProbeIds.remove(id)
          state.resolutions.remove(id)
        }
        true
      }
    }

    if (startsNewGeneration) {
      drain()
    }
  }

  suspend fun onConnectivityRestored(generation: Long) {
    val startsNewGeneration = stateMutex.withLock {
      if (generation <= lastConnectivityGeneration) {
        false
      } else {
        lastConnectivityGeneration = generation
        for (id in referencedIds) {
          when (state.resolutions[id]) {
            EditorAssetResolution.RetryableFailure -> state.resolutions.remove(id)
            EditorAssetResolution.Unavailable -> recoveryProbeIds.add(id)
            else -> Unit
          }
        }
        true
      }
    }

    if (startsNewGeneration) {
      drain()
    }
  }

  private suspend fun updateReferences(ids: Collection<String>) {
    val nextIds = ids.toSet()
    stateMutex.withLock {
      for (removedId in referencedIds - nextIds) {
        state.resolutions.remove(removedId)
        materializationAttempts.remove(removedId)
        recoveryProbeIds.remove(removedId)
      }
      referencedIds = nextIds
    }
  }

  private suspend fun drain() {
    resolverMutex.withLock {
      while (true) {
        val request = stateMutex.withLock { nextRequest() } ?: return
        val assets =
          try {
            fetch(request.ids)
          } catch (error: CancellationException) {
            clearTransientRequestState(request.ids)
            throw error
          } catch (error: Throwable) {
            val generationChanged = markFetchFailure(request)
            if (generationChanged) {
              continue
            }
            Logger.w(error) { "Editor asset hydration failed" }
            return
          }

        val retry = stateMutex.withLock { mergeResponse(request, assets) }
        if (retry != null) {
          try {
            waitBeforeRetry(retry.attempts.values.max())
          } catch (error: CancellationException) {
            clearTransientRequestState(retry.attempts.keys)
            throw error
          }
          stateMutex.withLock {
            for ((id, attempt) in retry.attempts) {
              if (state.resolutions[id] == EditorAssetResolution.AwaitingMaterialization(attempt)) {
                state.resolutions.remove(id)
              }
            }
          }
        }
      }
    }
  }

  private fun nextRequest(): Request? {
    val ids =
      referencedIds
        .asSequence()
        .filterNot(state::containsAsset)
        .filter { id -> recoveryProbeIds.contains(id) || state.resolutions[id] == null }
        .sorted()
        .take(MaxBatchSize)
        .toList()
    if (ids.isEmpty()) {
      return null
    }

    ids.forEach { id -> state.resolutions[id] = EditorAssetResolution.InFlight }
    return Request(
      ids = ids,
      queryGeneration = queryGeneration,
      connectivityGeneration = lastConnectivityGeneration,
      recoveryIds = ids.filterTo(mutableSetOf(), recoveryProbeIds::contains),
    )
  }

  private suspend fun clearTransientRequestState(ids: Collection<String>) {
    stateMutex.withLock {
      for (id in ids) {
        when (state.resolutions[id]) {
          EditorAssetResolution.InFlight,
          is EditorAssetResolution.AwaitingMaterialization -> state.resolutions.remove(id)
          else -> Unit
        }
      }
    }
  }

  private suspend fun markFetchFailure(request: Request): Boolean = stateMutex.withLock {
    if (request.isObsolete()) {
      request.ids.forEach { id ->
        if (state.resolutions[id] == EditorAssetResolution.InFlight) {
          state.resolutions.remove(id)
        }
      }
      true
    } else {
      request.ids.forEach { id ->
        recoveryProbeIds.remove(id)
        if (id in referencedIds && !state.containsAsset(id)) {
          state.resolutions[id] = EditorAssetResolution.RetryableFailure
        } else {
          state.resolutions.remove(id)
        }
      }
      false
    }
  }

  private fun mergeResponse(request: Request, assets: Collection<EditorExternalAsset>): Retry? {
    val requestedIds = request.ids.toSet()
    val returnedIds = mutableSetOf<String>()
    for (asset in assets) {
      if (asset.id !in requestedIds) {
        continue
      }
      returnedIds.add(asset.id)
      state.put(asset)
      state.resolutions.remove(asset.id)
      materializationAttempts.remove(asset.id)
      recoveryProbeIds.remove(asset.id)
    }

    if (request.isObsolete()) {
      request.ids.forEach { id ->
        if (id !in returnedIds && state.resolutions[id] == EditorAssetResolution.InFlight) {
          state.resolutions.remove(id)
        }
      }
      return null
    }

    val retryAttempts = mutableMapOf<String, Int>()
    for (id in request.ids) {
      if (id in returnedIds) {
        continue
      }
      if (id !in referencedIds) {
        state.resolutions.remove(id)
        materializationAttempts.remove(id)
        recoveryProbeIds.remove(id)
        continue
      }
      if (id in request.recoveryIds) {
        recoveryProbeIds.remove(id)
        state.resolutions[id] = EditorAssetResolution.Unavailable
        continue
      }

      val attempt = (materializationAttempts[id] ?: 0) + 1
      materializationAttempts[id] = attempt
      if (attempt >= MaxMaterializationAttempts) {
        state.resolutions[id] = EditorAssetResolution.Unavailable
      } else {
        state.resolutions[id] = EditorAssetResolution.AwaitingMaterialization(attempt)
        retryAttempts[id] = attempt
      }
    }

    return if (retryAttempts.isEmpty()) {
      null
    } else {
      Retry(attempts = retryAttempts)
    }
  }

  private fun Request.isObsolete(): Boolean =
    queryGeneration != this@EditorAssetHydrator.queryGeneration ||
      connectivityGeneration != lastConnectivityGeneration

  private fun mergeAssets(assets: Collection<EditorExternalAsset>) {
    for (asset in assets) {
      state.put(asset)
      state.resolutions.remove(asset.id)
      materializationAttempts.remove(asset.id)
      recoveryProbeIds.remove(asset.id)
    }
  }

  private companion object {
    const val MaxBatchSize = 50
    const val MaxMaterializationAttempts = 3
  }
}
