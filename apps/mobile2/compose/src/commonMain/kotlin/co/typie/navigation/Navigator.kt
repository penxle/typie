package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateListOf
import androidx.lifecycle.ViewModelStore
import co.typie.route.Route

enum class NavOperation { None, Push, Pop }

class Navigator(startRoute: Route) {
  private val _stack = mutableStateListOf(startRoute)
  val stack: List<Route> get() = _stack

  private val _modals = mutableStateListOf<@Composable () -> Unit>()
  val modals: List<@Composable () -> Unit> get() = _modals

  private val viewModelStores = mutableMapOf<Route, ViewModelStore>()

  val current: Route get() = _stack.last()
  val previous: Route? get() = if (_stack.size > 1) _stack[_stack.lastIndex - 1] else null
  val canPop: Boolean get() = _stack.size > 1 || _modals.isNotEmpty()

  var lastOperation: NavOperation = NavOperation.None
    private set

  fun viewModelStoreFor(route: Route): ViewModelStore {
    return viewModelStores.getOrPut(route) { ViewModelStore() }
  }

  fun navigate(route: Route) {
    _stack.add(route)
    lastOperation = NavOperation.Push
  }

  fun pop(): Boolean {
    if (_modals.isNotEmpty()) return dismissModal()
    if (_stack.size <= 1) return false
    val removed = _stack.removeLast()
    viewModelStores.remove(removed)?.clear()
    lastOperation = NavOperation.Pop
    return true
  }

  fun popTo(route: Route) {
    val index = _stack.lastIndexOf(route)
    if (index < 0) return
    while (_stack.size > index + 1) {
      val removed = _stack.removeLast()
      viewModelStores.remove(removed)?.clear()
    }
  }

  fun popToRoot() {
    while (_stack.size > 1) {
      val removed = _stack.removeLast()
      viewModelStores.remove(removed)?.clear()
    }
  }

  fun clear() {
    viewModelStores.values.forEach { it.clear() }
    viewModelStores.clear()
  }

  fun showModal(content: @Composable () -> Unit) {
    _modals.add(content)
  }

  fun dismissModal(): Boolean {
    if (_modals.isEmpty()) return false
    _modals.removeLast()
    return true
  }
}
