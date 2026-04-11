package co.typie.storage

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.ExperimentalForInheritanceCoroutinesApi
import kotlinx.coroutines.flow.FlowCollector
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

@OptIn(ExperimentalForInheritanceCoroutinesApi::class)
class PersistentStateFlow<T>(initialValue: T, private val persist: (T) -> Unit) :
  MutableStateFlow<T> {
  private val delegate = MutableStateFlow(initialValue)

  override var value: T
    get() = delegate.value
    set(newValue) {
      val old = delegate.value
      delegate.value = newValue
      if (old != newValue) persist(newValue)
    }

  override fun compareAndSet(expect: T, update: T): Boolean {
    val result = delegate.compareAndSet(expect, update)
    if (result) persist(update)
    return result
  }

  override suspend fun emit(value: T) {
    this.value = value
  }

  override fun tryEmit(value: T): Boolean {
    this.value = value
    return true
  }

  override val subscriptionCount: StateFlow<Int>
    get() = delegate.subscriptionCount

  override val replayCache: List<T>
    get() = delegate.replayCache

  @OptIn(ExperimentalCoroutinesApi::class)
  override fun resetReplayCache() = delegate.resetReplayCache()

  override suspend fun collect(collector: FlowCollector<T>): Nothing = delegate.collect(collector)
}
