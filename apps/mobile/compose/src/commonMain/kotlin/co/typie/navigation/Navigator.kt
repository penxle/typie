package co.typie.navigation

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

sealed interface NavigationResult {
  data object ReachedTarget : NavigationResult

  data class StoppedAt(val route: Route) : NavigationResult

  data object NotStarted : NavigationResult
}

class Navigator(startRoute: Route) {
  internal constructor(stack: List<Route>) : this(stack.first()) {
    _stack.addAll(stack.drop(1))
  }

  private val _stack = mutableStateListOf(startRoute)
  val stack: List<Route>
    get() = _stack

  private val viewModelStores = mutableMapOf<Route, ViewModelStore>()
  internal val routeRemovals = RouteRemovalCoordinator()

  val current: Route
    get() = _stack.last()

  val previous: Route?
    get() = if (_stack.size > 1) _stack[_stack.lastIndex - 1] else null

  val canPop: Boolean
    get() = _stack.size > 1

  var lastOperation: NavOperation = NavOperation.None
    private set

  private var pendingPopTarget: Route? by mutableStateOf(null)
  val popRequested: Boolean
    get() = pendingPopTarget != null

  private var transitionCompletion: CompletableDeferred<NavigationResult>? = null

  val isTransitioning: Boolean
    get() = transitionCompletion?.isActive == true

  fun viewModelStoreFor(route: Route): ViewModelStore {
    return viewModelStores.getOrPut(route) { ViewModelStore() }
  }

  suspend fun navigate(route: Route): NavigationResult {
    if (route == current) return resultForCurrentRoute()
    // 이미 스택에 있는 Route면 해당 위치까지 pop
    val existingIndex = _stack.indexOf(route)
    if (existingIndex >= 0) {
      return popTo(route)
    }
    if (isTransitioning) return NavigationResult.NotStarted
    val deferred = CompletableDeferred<NavigationResult>()
    transitionCompletion = deferred
    _stack.add(route)
    lastOperation = NavOperation.Push
    return deferred.await()
  }

  suspend fun pop(): NavigationResult =
    previous?.let { target -> requestRemoval(target) } ?: NavigationResult.ReachedTarget

  internal fun completeTransition(
    error: Throwable? = null,
    result: NavigationResult = NavigationResult.ReachedTarget,
  ) {
    if (error == null) {
      transitionCompletion?.complete(result)
    } else {
      transitionCompletion?.completeExceptionally(error)
    }
    transitionCompletion = null
  }

  internal fun consumePopRequest() {
    pendingPopTarget = null
  }

  internal fun peekPopTarget(): Route? = pendingPopTarget

  internal fun performPopTo(route: Route): List<Route> {
    val index = _stack.lastIndexOf(route)
    if (index < 0) return emptyList()
    val removedRoutes = mutableListOf<Route>()
    while (_stack.size > index + 1) {
      val removed = _stack.removeAt(_stack.lastIndex)
      viewModelStores.remove(removed)?.clear()
      removedRoutes += removed
    }
    if (removedRoutes.isNotEmpty()) {
      lastOperation = NavOperation.Pop
    }
    return removedRoutes
  }

  suspend fun popTo(route: Route): NavigationResult {
    val index = _stack.lastIndexOf(route)
    if (index < 0) return NavigationResult.NotStarted
    if (index == _stack.lastIndex) return resultForCurrentRoute()
    return requestRemoval(route)
  }

  suspend fun popToRoot(): NavigationResult = popTo(_stack.first())

  private suspend fun requestRemoval(target: Route): NavigationResult {
    val activeTransition = transitionCompletion
    if (activeTransition?.isActive == true) {
      return if (popRequested && pendingPopTarget == target) {
        activeTransition.await()
      } else {
        NavigationResult.NotStarted
      }
    }
    val deferred = CompletableDeferred<NavigationResult>()
    transitionCompletion = deferred
    pendingPopTarget = target
    return deferred.await()
  }

  private suspend fun resultForCurrentRoute(): NavigationResult {
    val activeTransition = transitionCompletion
    return when {
      activeTransition?.isActive != true -> NavigationResult.ReachedTarget
      !popRequested -> activeTransition.await()
      else -> NavigationResult.NotStarted
    }
  }

  fun clear() {
    viewModelStores.values.forEach { it.clear() }
    viewModelStores.clear()
  }
}
