package co.typie.navigation

import co.typie.route.Route
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext

internal enum class RouteRemovalPreparation {
  Ready,
  NeedsDecision,
}

internal enum class RouteRemovalDecision {
  CancelRemoval,
  ProceedWithRemoval,
}

internal interface RouteRemovalInterceptor {
  suspend fun prepare(onDelayed: (suspend () -> Unit)? = null): RouteRemovalPreparation

  suspend fun resolveDecision(): RouteRemovalDecision

  suspend fun rollback()
}

internal data class PreparedRemovalSegment(val destination: Route, val blockedRoute: Route?)

/** Coordinates route-local leave work without teaching Navigator about route-specific state. */
internal class RouteRemovalCoordinator {
  private data class RegisteredInterceptor(val interceptor: RouteRemovalInterceptor)

  private data class PreparedRoute(val route: Route, val registration: RegisteredInterceptor)

  private val interceptors = mutableMapOf<Route, RegisteredInterceptor>()
  private val approvedRoutes = mutableListOf<PreparedRoute>()
  private val preparedRoutes = mutableListOf<PreparedRoute>()
  private var preparingRoute: PreparedRoute? = null
  private var blockedRoute: PreparedRoute? = null

  fun register(route: Route, interceptor: RouteRemovalInterceptor): () -> Unit {
    val registration = RegisteredInterceptor(interceptor)
    interceptors[route] = registration
    return {
      if (interceptors[route] === registration) {
        interceptors.remove(route)
      }
    }
  }

  fun activeSegmentIsCurrent(): Boolean {
    val preparedCurrent = preparedRoutes.all { interceptors[it.route] === it.registration }
    val approvedRoutesCurrent = approvedRoutes.all { interceptors[it.route] === it.registration }
    val preparing = preparingRoute
    val blocked = blockedRoute
    return preparedCurrent &&
      approvedRoutesCurrent &&
      (preparing == null || interceptors[preparing.route] === preparing.registration) &&
      (blocked == null || interceptors[blocked.route] === blocked.registration)
  }

  fun hasInterceptor(route: Route): Boolean = route in interceptors

  suspend fun prepareSegment(
    routesToRemove: List<Route>,
    requestedTarget: Route,
    onDelayed: (suspend (Route) -> Unit)? = null,
  ): PreparedRemovalSegment {
    check(preparedRoutes.isEmpty() && preparingRoute == null && blockedRoute == null) {
      "A route removal segment is already active"
    }

    try {
      routesToRemove.forEach { route ->
        val registration = interceptors[route] ?: return@forEach
        val approved = approvedRoutes.firstOrNull { it.route == route }
        if (approved != null) {
          if (approved.registration === registration) return@forEach
          approvedRoutes.remove(approved)
          rollbackRoutes(listOf(approved))?.let { throw it }
        }

        val preparedRoute = PreparedRoute(route, registration)
        preparingRoute = preparedRoute
        val preparation =
          try {
            registration.interceptor.prepare(onDelayed?.let { callback -> { callback(route) } })
          } catch (throwable: Throwable) {
            if (preparingRoute === preparedRoute) preparingRoute = null
            throwable.addCleanupFailure(rollbackRoutes(listOf(preparedRoute)))
            throw throwable
          }
        if (preparingRoute === preparedRoute) preparingRoute = null
        when (preparation) {
          RouteRemovalPreparation.Ready -> preparedRoutes += preparedRoute
          RouteRemovalPreparation.NeedsDecision -> {
            blockedRoute = preparedRoute
            return PreparedRemovalSegment(destination = route, blockedRoute = route)
          }
        }
      }
    } catch (throwable: Throwable) {
      throwable.addCleanupFailure(rollbackPreparedRoutes())
      throw throwable
    }

    return PreparedRemovalSegment(destination = requestedTarget, blockedRoute = null)
  }

  /** Called after the routes above a blocked route have actually left the stack. */
  fun commitReadyPrefix() {
    preparedRoutes.clear()
    approvedRoutes.clear()
  }

  /** Called after an unblocked segment has actually left the stack. */
  fun commitSegment() {
    check(preparingRoute == null) { "A route removal preparation is still active" }
    preparedRoutes.clear()
    blockedRoute = null
    approvedRoutes.clear()
  }

  suspend fun resolveBlockedRoute(): NavigationResult? {
    val blocked = checkNotNull(blockedRoute) { "No route is awaiting a removal decision" }
    val currentRegistration = interceptors[blocked.route]
    if (currentRegistration !== blocked.registration) {
      blockedRoute = null
      rollbackRoutes(listOf(blocked))?.let { throw it }
      return null
    }
    val decision = blocked.registration.interceptor.resolveDecision()

    return when (decision) {
      RouteRemovalDecision.CancelRemoval -> {
        blockedRoute = null
        approvedRoutes.clear()
        rollbackRoutes(listOf(blocked))?.let { throw it }
        NavigationResult.StoppedAt(blocked.route)
      }
      RouteRemovalDecision.ProceedWithRemoval -> {
        blockedRoute = null
        approvedRoutes += blocked
        null
      }
    }
  }

  suspend fun rollbackActiveSegment() {
    val routes = buildList {
      blockedRoute?.let(::add)
      preparingRoute?.let(::add)
      addAll(preparedRoutes.asReversed())
      addAll(approvedRoutes.asReversed())
    }
    blockedRoute = null
    preparingRoute = null
    preparedRoutes.clear()
    approvedRoutes.clear()
    rollbackRoutes(routes)?.let { throw it }
  }

  private suspend fun rollbackPreparedRoutes(): Throwable? {
    val routes = preparedRoutes.asReversed().toList()
    preparedRoutes.clear()
    return rollbackRoutes(routes)
  }

  private suspend fun rollbackRoutes(routes: List<PreparedRoute>): Throwable? =
    withContext(NonCancellable) {
      var firstFailure: Throwable? = null
      routes.forEach { prepared ->
        try {
          prepared.registration.interceptor.rollback()
        } catch (throwable: Throwable) {
          val previousFailure = firstFailure
          if (previousFailure == null) {
            firstFailure = throwable
          } else if (throwable !== previousFailure) {
            previousFailure.addSuppressed(throwable)
          }
        }
      }
      firstFailure
    }

  private fun Throwable.addCleanupFailure(failure: Throwable?) {
    if (failure != null && failure !== this) addSuppressed(failure)
  }
}
