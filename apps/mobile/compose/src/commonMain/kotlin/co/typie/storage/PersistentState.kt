package co.typie.storage

import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf

class PersistentState<T>(initialValue: T, private val persist: (T) -> Unit) : MutableState<T> {
  private val delegate = mutableStateOf(initialValue)

  override var value: T
    get() = delegate.value
    set(newValue) {
      val old = delegate.value
      delegate.value = newValue
      if (old != newValue) persist(newValue)
    }

  override fun component1() = value

  override fun component2(): (T) -> Unit = { value = it }
}
