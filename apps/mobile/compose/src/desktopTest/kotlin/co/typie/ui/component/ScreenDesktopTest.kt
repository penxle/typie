package co.typie.ui.component

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.route.Route
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.TopBarState
import kotlin.test.Test
import kotlin.test.assertEquals

private const val ContentTag = "screen-content"
private const val OverlayTag = "screen-overlay"
private val LocalScreenOverlayTestValue = staticCompositionLocalOf { "default" }

@OptIn(ExperimentalTestApi::class)
class ScreenDesktopTest {
  @Test
  fun rendersWithoutNavigatorProviderWhenLoadableIsAbsent() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(LocalDialog provides Dialog()) {
        Screen { Box(Modifier.testTag(ContentTag)) }
      }
    }
    waitForIdle()

    onNodeWithTag(ContentTag).assertExists()
  }

  @Test
  fun overlayRendersLocallyAtScreenCoordinatesWithoutNavigationHost() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(LocalDialog provides Dialog()) {
        Screen(
          overlay = { Box(Modifier.fillMaxSize().testTag(OverlayTag)) },
          content = { Box(Modifier.fillMaxSize().testTag(ContentTag)) },
        )
      }
    }
    waitForIdle()

    assertEquals(Offset.Zero, onNodeWithTag(ContentTag).fetchSemanticsNode().boundsInRoot.topLeft)
    assertEquals(Offset.Zero, onNodeWithTag(OverlayTag).fetchSemanticsNode().boundsInRoot.topLeft)
  }

  @Test
  fun topContentPaddingEndsAtPhysicalTopBarBoundary() = runComposeUiTest {
    val topBarState =
      TopBarState().apply {
        enabled = true
        visible = true
      }
    var actualTopPadding = Dp.Unspecified
    var expectedTopPadding = Dp.Unspecified

    setContent {
      expectedTopPadding = TopBarDefaults.topPadding() + TopBarDefaults.Height
      CompositionLocalProvider(
        LocalDialog provides Dialog(),
        LocalTopBarState provides topBarState,
      ) {
        Screen { contentPadding ->
          actualTopPadding = contentPadding.calculateTopPadding()
          Box(Modifier.fillMaxSize().testTag(ContentTag))
        }
      }
    }
    waitForIdle()

    assertEquals(expectedTopPadding, actualTopPadding)
  }

  @Test
  fun navigationHostsScreenOverlayOnceWithDeclarationContext() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var attachCount = 0
    var observedContext = ""

    setContent {
      CompositionLocalProvider(LocalDialog provides Dialog()) {
        NavigationStack(
          navigator = navigator,
          topBarState = TopBarState(),
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) { route ->
          if (route == editorRoute) {
            CompositionLocalProvider(LocalScreenOverlayTestValue provides "context-$route") {
              Screen(
                overlay = {
                  observedContext = LocalScreenOverlayTestValue.current
                  DisposableEffect(Unit) {
                    attachCount += 1
                    onDispose {}
                  }
                  Box(Modifier.fillMaxSize().testTag("$OverlayTag-$route"))
                },
                content = { Box(Modifier.fillMaxSize().testTag("$ContentTag-$route")) },
              )
            }
          } else {
            Box(Modifier.fillMaxSize().testTag("$ContentTag-$route"))
          }
        }
        LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
      }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    onAllNodes(hasTestTag("$OverlayTag-$editorRoute")).assertCountEquals(1)
    assertEquals(1, attachCount)
    assertEquals("context-$editorRoute", observedContext)
    assertEquals(
      onNodeWithTag("$ContentTag-$editorRoute").fetchSemanticsNode().boundsInRoot.topLeft,
      onNodeWithTag("$OverlayTag-$editorRoute").fetchSemanticsNode().boundsInRoot.topLeft,
    )
  }
}
