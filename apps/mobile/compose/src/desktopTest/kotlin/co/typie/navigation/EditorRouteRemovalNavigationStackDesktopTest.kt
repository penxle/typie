package co.typie.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.DocumentEditingClose
import co.typie.editor.EditingCheckpointResult
import co.typie.editor.EditorLocalEditCoordinator
import co.typie.route.Route
import co.typie.screen.editor.editor.EditorRouteLeaveInterceptor
import co.typie.ui.component.topbar.TopBarState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlinx.coroutines.CompletableDeferred

@OptIn(ExperimentalTestApi::class)
class EditorRouteRemovalNavigationStackDesktopTest {
  @Test
  fun editorIsRemovedOnlyAfterLocalEditBarrierAndCaptureComplete() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("editor")
    val localEdits = EditorLocalEditCoordinator()
    val pendingEdit = checkNotNull(localEdits.register())
    val popRequested = CompletableDeferred<Unit>()
    val barrierStarted = CompletableDeferred<Unit>()
    val captureStarted = CompletableDeferred<Unit>()
    val releaseCapture = CompletableDeferred<Unit>()
    var popResult: NavigationResult? = null

    navigator.routeRemovals.register(
      editorRoute,
      EditorRouteLeaveInterceptor(
        finalizeInput = {},
        restoreInput = {},
        beginClose = {
          val quiescence = localEdits.quiesce()
          object : DocumentEditingClose {
            override suspend fun awaitCheckpoint(): EditingCheckpointResult {
              barrierStarted.complete(Unit)
              val editResult = quiescence.await()
              captureStarted.complete(Unit)
              releaseCapture.await()
              return editResult.fold(
                onSuccess = { EditingCheckpointResult.Protected },
                onFailure = { EditingCheckpointResult.EditFailed(it) },
              )
            }

            override fun cancel() {
              quiescence.resume()
            }
          }
        },
        resolveDecision = { error("Successful capture must not require a decision") },
      ),
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
      LaunchedEffect(Unit) {
        navigator.navigate(editorRoute)
        popRequested.await()
        popResult = navigator.pop()
      }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    popRequested.complete(Unit)
    waitUntil { barrierStarted.isCompleted }
    assertFalse(captureStarted.isCompleted)
    assertEquals(editorRoute, navigator.current)
    assertNull(popResult)

    localEdits.complete(pendingEdit)
    waitUntil { captureStarted.isCompleted }
    assertEquals(editorRoute, navigator.current)
    assertNull(popResult)

    releaseCapture.complete(Unit)
    waitUntil { navigator.current == Route.Home && popResult != null }
    assertEquals(NavigationResult.ReachedTarget, popResult)
  }
}
