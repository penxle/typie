package co.typie.service

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.channelFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.filter

internal const val DEFAULT_SITE_REFRESH_DEBOUNCE_MS = 200L

internal fun Flow<String>.coalescedSiteRefreshes(
  siteId: String,
  debounceMs: Long = DEFAULT_SITE_REFRESH_DEBOUNCE_MS,
): Flow<Unit> {
  return filter { it == siteId }
    .coalescedRefreshSignals(delayMillis = debounceMs)
}

object SiteRefreshCoordinator {
  private val signals = MutableSharedFlow<String>(
    extraBufferCapacity = 64,
    onBufferOverflow = BufferOverflow.DROP_OLDEST,
  )

  fun notifySiteChanged(siteId: String) {
    if (siteId.isBlank()) {
      return
    }

    signals.tryEmit(siteId)
  }

  fun refreshes(siteId: String): Flow<Unit> {
    if (siteId.isBlank()) {
      return emptyFlow()
    }

    return signals.coalescedSiteRefreshes(siteId)
  }
}

private fun Flow<String>.coalescedRefreshSignals(
  delayMillis: Long,
): Flow<Unit> {
  return channelFlow {
    this@coalescedRefreshSignals.collectLatest {
      delay(delayMillis)
      send(Unit)
    }
  }
}
