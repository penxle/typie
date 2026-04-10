package co.typie.graphql

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import co.typie.navigation.LocalRoute
import co.typie.navigation.Nav
import co.typie.service.SiteRefreshCoordinator
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.distinctUntilChanged

@Composable
fun RefetchOnScreenEnterEffect(onScreenEntered: () -> Unit) {
  val nav = Nav.current
  val route = LocalRoute.current
  val latestOnScreenEntered by rememberUpdatedState(onScreenEntered)

  LaunchedEffect(nav, route) {
    snapshotFlow { nav.current == route }
      .distinctUntilChanged()
      .collect { isVisible ->
        if (isVisible) {
          latestOnScreenEntered()
        }
      }
  }
}

@Composable
fun RefetchOnAppResumeEffect(onResume: () -> Unit) {
  val lifecycleOwner = LocalLifecycleOwner.current
  val nav = Nav.current
  val route = LocalRoute.current
  val latestOnResume by rememberUpdatedState(onResume)

  DisposableEffect(lifecycleOwner, nav, route) {
    val observer = LifecycleEventObserver { _, event ->
      if (event == Lifecycle.Event.ON_RESUME && nav.current == route) {
        latestOnResume()
      }
    }

    lifecycleOwner.lifecycle.addObserver(observer)
    onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
  }
}

@Composable
fun RefetchOnSiteUpdateEffect(siteId: String, onRefetch: () -> Unit) {
  val nav = Nav.current
  val route = LocalRoute.current
  val siteRefreshCoordinator = SiteRefreshCoordinator
  val latestOnRefetch by rememberUpdatedState(onRefetch)

  LaunchedEffect(siteId, nav, route) {
    if (siteId.isBlank()) {
      return@LaunchedEffect
    }

    siteRefreshCoordinator.refreshes(siteId).collect {
      if (nav.current == route) {
        latestOnRefetch()
      }
    }
  }
}
