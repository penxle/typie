package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModelStore
import co.typie.route.Route
import kotlinx.coroutines.CompletableDeferred

enum class NavOperation {
  None,
  Push,
  Pop,
}

class Navigator(startRoute: Route) {
  private val _stack = mutableStateListOf(startRoute)
  val stack: List<Route>
    get() = _stack

  private val _modals = mutableStateListOf<@Composable () -> Unit>()
  val modals: List<@Composable () -> Unit>
    get() = _modals

  private val viewModelStores = mutableMapOf<Route, ViewModelStore>()

  val current: Route
    get() = _stack.last()

  val previous: Route?
    get() = if (_stack.size > 1) _stack[_stack.lastIndex - 1] else null

  val canPop: Boolean
    get() = _stack.size > 1 || _modals.isNotEmpty()

  var lastOperation: NavOperation = NavOperation.None
    private set

  var popRequested: Boolean by mutableStateOf(false)
    private set

  private var transitionCompletion: CompletableDeferred<Unit>? = null

  val isTransitioning: Boolean
    get() = transitionCompletion?.isActive == true

  fun viewModelStoreFor(route: Route): ViewModelStore {
    return viewModelStores.getOrPut(route) { ViewModelStore() }
  }

  suspend fun navigate(route: Route) {
    if (isTransitioning) return
    if (route == current) return
    // 이미 스택에 있는 Route면 해당 위치까지 pop
    val existingIndex = _stack.indexOf(route)
    if (existingIndex >= 0) {
      // 중간 Route 제거 (target과 current만 남김)
      while (_stack.size > existingIndex + 2) {
        val removed = _stack.removeAt(existingIndex + 1)
        viewModelStores.remove(removed)?.clear()
      }
      pop()
      return
    }
    val deferred = CompletableDeferred<Unit>()
    transitionCompletion = deferred
    _stack.add(route)
    lastOperation = NavOperation.Push
    deferred.await()
  }

  suspend fun pop() {
    if (!canPop) return
    if (isTransitioning) return
    val deferred = CompletableDeferred<Unit>()
    transitionCompletion = deferred
    popRequested = true
    deferred.await()
  }

  internal fun requestPop() {
    if (canPop) popRequested = true
  }

  internal fun completeTransition() {
    transitionCompletion?.complete(Unit)
    transitionCompletion = null
  }

  internal fun consumePopRequest() {
    popRequested = false
  }

  internal fun performPop(): Boolean {
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
