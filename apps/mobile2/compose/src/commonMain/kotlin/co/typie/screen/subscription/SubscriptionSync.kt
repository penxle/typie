package co.typie.screen.subscription

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import org.koin.core.annotation.Single

@Single
class SubscriptionSync {
  private val _events = MutableSharedFlow<Int>(
    replay = 0,
    extraBufferCapacity = 8,
  )
  val events = _events.asSharedFlow()

  private var nextEventId = 0

  fun notifyChanged() {
    nextEventId += 1
    _events.tryEmit(nextEventId)
  }
}
