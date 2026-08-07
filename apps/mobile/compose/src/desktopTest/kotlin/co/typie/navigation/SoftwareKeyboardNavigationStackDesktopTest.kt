package co.typie.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard
import co.typie.dev.ProvideDesktopDebugKeyboardPresentation
import co.typie.ext.TextInputClient
import co.typie.ext.ime
import co.typie.platform.LocalSoftwareKeyboardPresentationController
import co.typie.platform.ProvideSoftwareKeyboardPresentation
import co.typie.platform.SoftwareKeyboardInteractionResolution
import co.typie.platform.SoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationDriver
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import co.typie.route.Route
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class SoftwareKeyboardNavigationStackDesktopTest {
  @Test
  fun committedRouteRetainsItsOutgoingCompositionUntilHiddenResolves() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("software-keyboard-async-resolution")
    val driver = DeferredEndpointDriver()
    val controller = SoftwareKeyboardPresentationController { driver }
    var activationDistancePx = 0f
    var editorAttached = false

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalSoftwareKeyboardPresentationController provides controller,
      ) {
        activationDistancePx =
          LocalViewConfiguration.current.touchSlop * NAVIGATION_POP_ACTIVATION_SLOP_MULTIPLIER
        NavigationStackTestHost(
          navigator = navigator,
          topBarState = remember { TopBarState() },
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) { route ->
          if (route == editorRoute) {
            DisposableEffect(Unit) {
              editorAttached = true
              onDispose { editorAttached = false }
            }
          }
          Box(
            Modifier.fillMaxSize()
              .testTag(if (route == editorRoute) EDITOR_ROUTE_TAG else HOME_ROUTE_TAG)
          )
        }
        LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
      }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    onNodeWithTag(EDITOR_ROUTE_TAG).performTouchInput {
      down(Offset(x = 10f, y = center.y))
      moveBy(Offset(x = activationDistancePx + 320f, y = 0f), delayMillis = 600L)
      up()
    }
    waitUntil {
      navigator.current == Route.Home &&
        driver.endpoint == SoftwareKeyboardPresentationEndpoint.Hidden
    }

    assertTrue(editorAttached)

    runOnIdle { driver.acceptEndpoint() }
    waitUntil { !navigator.isTransitioning && !editorAttached }
  }

  @Test
  fun backSwipeRestoresTheKeyboardWhenCancelledAndHidesItWhenCommitted() = runComposeUiTest {
    val owner = Any()
    val client = KeyboardTestClient()
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("software-keyboard-navigation")
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var keyboardPresentationController: SoftwareKeyboardPresentationController
    var activationDistancePx = 0f
    var imeBottomPx = 0f
    var imeBottomOffsetPx = 0f
    var unregisterInterceptor: (() -> Unit)? = null
    var rollbackCount = 0

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true)
      }
      setContent {
        CompositionLocalProvider(LocalAppColors provides LightColors) {
          ProvideSoftwareKeyboardPresentation {
            ProvideDesktopDebugKeyboardPresentation {
              val density = LocalDensity.current
              val currentKeyboardPresentationController =
                LocalSoftwareKeyboardPresentationController.current
              val imeBottom = WindowInsets.ime.getBottom(density).toFloat()
              activationDistancePx =
                LocalViewConfiguration.current.touchSlop * NAVIGATION_POP_ACTIVATION_SLOP_MULTIPLIER
              SideEffect {
                imeBottomPx = imeBottom
                imeBottomOffsetPx = with(density) { DESKTOP_IME_BOTTOM_OFFSET.toPx() }
                keyboardPresentationController = currentKeyboardPresentationController
              }

              Box(Modifier.size(width = 320.dp, height = 640.dp).testTag(ROOT_TAG)) {
                NavigationStackTestHost(
                  navigator = navigator,
                  topBarState = remember { TopBarState() },
                  modifier = Modifier.fillMaxSize(),
                ) { route ->
                  Box(
                    Modifier.fillMaxSize()
                      .testTag(if (route == editorRoute) EDITOR_ROUTE_TAG else HOME_ROUTE_TAG)
                  )
                }
                DesktopDebugKeyboard.Overlay(
                  Modifier.align(Alignment.BottomStart).testTag(KEYBOARD_TAG)
                )
              }
            }
          }
        }
        LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
      }
      waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

      val routeNode = onNodeWithTag(EDITOR_ROUTE_TAG)
      val fullKeyboardHeight = onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height
      val fullImeBottom = imeBottomPx

      routeNode.performTouchInput {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = activationDistancePx - 1f, y = 0f), delayMillis = 100L)
      }
      waitForIdle()

      assertEquals(
        fullKeyboardHeight,
        onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height,
        absoluteTolerance = GEOMETRY_TOLERANCE_PX,
      )
      assertEquals(fullImeBottom, imeBottomPx, absoluteTolerance = GEOMETRY_TOLERANCE_PX)

      routeNode.performTouchInput { moveBy(Offset(x = 80f, y = 0f), delayMillis = 500L) }
      waitUntil {
        routeNode.fetchSemanticsNode().boundsInRoot.left > 1f &&
          onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height <
            fullKeyboardHeight - 1f
      }

      val routeProgress =
        routeNode.fetchSemanticsNode().boundsInRoot.left /
          onNodeWithTag(ROOT_TAG).fetchSemanticsNode().boundsInRoot.width
      val interactiveKeyboardHeight =
        onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height
      assertEquals(
        fullKeyboardHeight * (1f - routeProgress),
        interactiveKeyboardHeight,
        absoluteTolerance = GEOMETRY_TOLERANCE_PX,
      )
      assertEquals(
        interactiveKeyboardHeight,
        imeBottomPx - imeBottomOffsetPx,
        absoluteTolerance = GEOMETRY_TOLERANCE_PX,
      )

      routeNode.performTouchInput { up() }
      waitUntil {
        navigator.current == editorRoute &&
          !navigator.isTransitioning &&
          abs(
            onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height -
              fullKeyboardHeight
          ) < GEOMETRY_TOLERANCE_PX
      }

      assertEquals(fullImeBottom, imeBottomPx, absoluteTolerance = GEOMETRY_TOLERANCE_PX)
      assertTrue(DesktopDebugKeyboard.visible)

      unregisterInterceptor =
        navigator.routeRemovals.register(
          editorRoute,
          object : RouteRemovalInterceptor {
            override suspend fun prepare(
              onDelayed: (suspend () -> Unit)?
            ): RouteRemovalPreparation {
              checkNotNull(onDelayed).invoke()
              return RouteRemovalPreparation.NeedsDecision
            }

            override suspend fun resolveDecision(): RouteRemovalDecision =
              RouteRemovalDecision.CancelRemoval

            override suspend fun rollback() {
              rollbackCount += 1
            }
          },
        )
      routeNode.performTouchInput {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = activationDistancePx + 320f, y = 0f), delayMillis = 600L)
      }
      waitUntil {
        onAllNodes(hasTestTag(KEYBOARD_TAG)).fetchSemanticsNodes().isEmpty() && imeBottomPx == 0f
      }
      assertTrue(keyboardPresentationController.interactionState.unresolved)
      assertEquals(
        SoftwareKeyboardInteractionResolution.Shown,
        keyboardPresentationController.interactionState.lastResolution,
      )

      routeNode.performTouchInput { up() }
      waitUntil {
        rollbackCount == 1 &&
          navigator.current == editorRoute &&
          !navigator.isTransitioning &&
          abs(
            onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height -
              fullKeyboardHeight
          ) < GEOMETRY_TOLERANCE_PX
      }

      assertEquals(fullImeBottom, imeBottomPx, absoluteTolerance = GEOMETRY_TOLERANCE_PX)
      assertTrue(DesktopDebugKeyboard.visible)
      assertEquals(
        SoftwareKeyboardInteractionResolution.Shown,
        keyboardPresentationController.interactionState.lastResolution,
      )
      unregisterInterceptor.invoke()
      unregisterInterceptor = null

      mainClock.autoAdvance = false
      routeNode.performTouchInput {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = activationDistancePx + 256f, y = 0f), delayMillis = 600L)
      }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()
      waitForIdle()

      var previousKeyboardHeight =
        onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height
      routeNode.performTouchInput { up() }

      repeat(60) { frame ->
        mainClock.advanceTimeByFrame()
        waitForIdle()
        val currentKeyboardHeight =
          onAllNodes(hasTestTag(KEYBOARD_TAG))
            .fetchSemanticsNodes()
            .firstOrNull()
            ?.boundsInRoot
            ?.height ?: 0f
        assertTrue(
          currentKeyboardHeight <= previousKeyboardHeight + GEOMETRY_TOLERANCE_PX,
          "Keyboard rose from $previousKeyboardHeight to $currentKeyboardHeight after release at frame $frame",
        )
        previousKeyboardHeight = currentKeyboardHeight
      }

      assertEquals(Route.Home, navigator.current)
      assertFalse(navigator.isTransitioning)

      onNodeWithTag(KEYBOARD_TAG).assertDoesNotExist()
      assertEquals(0f, imeBottomPx)
      assertFalse(DesktopDebugKeyboard.visible)
      assertEquals(
        SoftwareKeyboardInteractionResolution.Hidden,
        keyboardPresentationController.interactionState.lastResolution,
      )
    } finally {
      mainClock.autoAdvance = true
      unregisterInterceptor?.invoke()
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }
}

private class DeferredEndpointDriver : SoftwareKeyboardPresentationDriver {
  var endpoint: SoftwareKeyboardPresentationEndpoint? = null
    private set

  private var acceptance: (() -> Unit)? = null

  override fun updateHiddenProgress(progress: Float) = Unit

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    this.endpoint = endpoint
    acceptance = onAccepted
  }

  override fun dispose() = Unit

  fun acceptEndpoint() {
    acceptance?.invoke()
  }
}

private class KeyboardTestClient : TextInputClient {
  override fun requestFocus() = Unit

  override fun dismiss() = Unit
}

private const val NAVIGATION_POP_ACTIVATION_SLOP_MULTIPLIER = 3f
private val DESKTOP_IME_BOTTOM_OFFSET = 12.dp
private const val GEOMETRY_TOLERANCE_PX = 1.5f
private const val ROOT_TAG = "software-keyboard-navigation-root"
private const val KEYBOARD_TAG = "software-keyboard-navigation-keyboard"
private const val EDITOR_ROUTE_TAG = "software-keyboard-navigation-editor"
private const val HOME_ROUTE_TAG = "software-keyboard-navigation-home"
