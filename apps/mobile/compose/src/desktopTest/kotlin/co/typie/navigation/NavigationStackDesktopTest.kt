package co.typie.navigation

import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.gestures.scrollable2D
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.TouchInjectionScope
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.route.Route
import co.typie.ui.component.topbar.TopBarState
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class NavigationStackDesktopTest {
  @Test
  fun verticalScrollSessionDoesNotBecomeBackSwipeAfterTurningRight() =
    assertGestureDoesNotPop(
      scrollConsumer = { delta -> if (abs(delta.y) > abs(delta.x)) delta else Offset.Zero },
      gesture = {
        down(center)
        moveBy(Offset(x = 0f, y = -120f))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  @Test
  fun leftNoOpSessionDoesNotBecomeBackSwipeAfterTurningRight() =
    assertGestureDoesNotPop(
      scrollConsumer = { Offset.Zero },
      gesture = {
        down(center)
        moveBy(Offset(x = -80f, y = 0f))
        moveBy(Offset(x = 240f, y = 0f))
        up()
      },
    )

  @Test
  fun horizontalScrollSessionDoesNotBecomeBackSwipeAtStartEdge() {
    var consumedHorizontalDrag = false
    assertGestureDoesNotPop(
      scrollConsumer = { delta ->
        if (!consumedHorizontalDrag && delta.x > 0f) {
          consumedHorizontalDrag = true
          delta
        } else {
          Offset.Zero
        }
      },
      gesture = {
        down(center)
        moveBy(Offset(x = 80f, y = 0f))
        moveBy(Offset(x = 160f, y = 0f))
        up()
      },
    )
  }

  @Test
  fun verticalEdgeSessionDoesNotBecomeBackSwipeAfterTurningRight() =
    assertGestureDoesNotPop(
      scrollConsumer = { delta -> if (abs(delta.y) > abs(delta.x)) delta else Offset.Zero },
      gesture = {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = 0f, y = -120f))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  @Test
  fun rejectedSessionAllowsBackSwipeAfterPointerUp() =
    assertGesturesPop(
      scrollConsumer = { delta -> if (abs(delta.y) > abs(delta.x)) delta else Offset.Zero },
      firstGesture = {
        down(center)
        moveBy(Offset(x = 0f, y = -120f))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
      secondGesture = {
        down(center)
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  private fun assertGestureDoesNotPop(
    scrollConsumer: (Offset) -> Offset,
    gesture: TouchInjectionScope.() -> Unit,
  ) = assertGestureResult(scrollConsumer, shouldPop = false, gesture)

  private fun assertGesturesPop(
    scrollConsumer: (Offset) -> Offset,
    firstGesture: TouchInjectionScope.() -> Unit,
    secondGesture: TouchInjectionScope.() -> Unit,
  ) = assertGestureResult(scrollConsumer, shouldPop = true, firstGesture, secondGesture)

  private fun assertGestureResult(
    scrollConsumer: (Offset) -> Offset,
    shouldPop: Boolean,
    vararg gestures: TouchInjectionScope.() -> Unit,
  ) = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        val scrollableState = rememberScrollable2DState(scrollConsumer)
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
            .navigationPopNestedScroll()
            .scrollable2D(scrollableState)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    gestures.forEach { gesture ->
      onNodeWithTag(EditorRouteTag).performTouchInput(gesture)
      waitForIdle()
    }

    assertEquals(if (shouldPop) Route.Home else editorRoute, navigator.current)
  }

  private companion object {
    const val EditorRouteTag = "editor-route"
    const val HomeRouteTag = "home-route"
  }
}
