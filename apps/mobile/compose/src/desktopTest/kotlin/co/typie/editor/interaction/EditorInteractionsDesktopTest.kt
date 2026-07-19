package co.typie.editor.interaction

import androidx.compose.foundation.MutatePriority
import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.ScrollScope
import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.pan
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.performTrackpadInput
import androidx.compose.ui.test.scale
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.ScrollGestureLockState
import co.typie.navigation.LocalNavigationPopNestedScroll
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.navigation.navigationPopNestedScroll
import co.typie.route.Route
import co.typie.ui.component.topbar.TopBarState
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.yield

@OptIn(ExperimentalTestApi::class)
class EditorInteractionsDesktopTest {
  @Test
  fun `desktop trackpad click drag scrolls the viewport like touch`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTrackpadInput {
      moveTo(Offset(x = 100f, y = 100f))
      press()
      moveBy(Offset(x = 4f, y = 0f))
      moveBy(Offset(x = 8f, y = 0f))
      release()
    }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.isNotEmpty())
  }

  @Test
  fun `editor physical click drag owns desktop scroll translation until release`() =
    runComposeUiTest {
      val fixture = Fixture()
      setEditorContent(fixture)

      onNodeWithTag(EditorTag).performTrackpadInput {
        moveTo(Offset(x = 100f, y = 100f))
        press()
      }
      waitForIdle()
      assertTrue(fixture.scrollGestureLockState.isLocked)

      onNodeWithTag(EditorTag).performTrackpadInput {
        moveBy(Offset(x = 20f, y = 0f))
        release()
      }
      waitForIdle()

      assertFalse(fixture.scrollGestureLockState.isLocked)
      assertTrue(fixture.touchPanDeltas.isNotEmpty())
    }

  @Test
  fun `desktop command pointer scroll zooms without moving navigation`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)
    val editor = onNodeWithTag(NavigationEditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performMouseInput { moveTo(center) }
    editor.performKeyInput { keyDown(Key.MetaLeft) }
    editor.performMouseInput { repeat(24) { scroll(Offset(x = 0f, y = -0.1f)) } }
    editor.performKeyInput { keyUp(Key.MetaLeft) }
    mainClock.advanceTimeBy(100)
    waitForIdle()

    assertTrue(fixture.zoomController.displayZoom > initialZoom)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = editor.fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `desktop ctrl pointer scroll remains viewport scroll`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)
    val editor = onNodeWithTag(EditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performMouseInput { moveTo(center) }
    editor.performKeyInput { keyDown(Key.CtrlLeft) }
    editor.performMouseInput { scroll(Offset(x = 0f, y = -96f)) }
    editor.performKeyInput { keyUp(Key.CtrlLeft) }
    waitForIdle()

    assertEquals(initialZoom, fixture.zoomController.displayZoom, 0.0001f)
    assertTrue(fixture.touchPanDeltas.isNotEmpty())
  }

  @Test
  fun `modified pointer scroll reaches editor before a child consumes it`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture, consumeIndirectInputAtChild = true)
    val editor = onNodeWithTag(EditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performMouseInput { moveTo(center) }
    editor.performKeyInput { keyDown(Key.MetaLeft) }
    editor.performMouseInput { scroll(Offset(x = 0f, y = -96f)) }
    editor.performKeyInput { keyUp(Key.MetaLeft) }
    waitForIdle()

    assertTrue(fixture.zoomController.displayZoom > initialZoom)
  }

  @Test
  fun `command scroll takes over a pressed tap and allows following scale`() = runComposeUiTest {
    val fixture = Fixture(tapEligible = true)
    setEditorContent(fixture)
    val editor = onNodeWithTag(EditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performMouseInput {
      moveTo(center)
      press()
    }
    waitForIdle()
    assertEquals(1, fixture.host.longPressDispatchScheduleCount)

    editor.performKeyInput { keyDown(Key.MetaLeft) }
    editor.performMouseInput { scroll(Offset(x = 0f, y = -24f)) }
    editor.performKeyInput { keyUp(Key.MetaLeft) }
    waitForIdle()

    assertTrue(fixture.host.longPressDispatchCancelCount > 0)
    assertTrue(fixture.zoomController.displayZoom > initialZoom)
    val zoomAfterScroll = fixture.zoomController.displayZoom
    runOnIdle {
      assertTrue(fixture.platformIndirectScaleBridge.begin())
      assertTrue(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.1f,
        )
      )
      fixture.platformIndirectScaleBridge.end()
    }
    assertTrue(fixture.zoomController.displayZoom > zoomAfterScroll)
    editor.performMouseInput { release() }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertFalse(fixture.scrollGestureLockState.isLocked)
  }

  @Test
  fun `common trackpad scale event does not start physical gestures`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)
    val initialZoom = fixture.zoomController.displayZoom

    onNodeWithTag(EditorTag).performTrackpadInput {
      updatePointerTo(Offset(x = 180f, y = 160f))
      scale(1.25f)
    }
    waitForIdle()

    assertEquals(initialZoom * 1.25f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertEquals(0, fixture.host.tapDispatchScheduleCount)
    assertEquals(0, fixture.host.longPressDispatchScheduleCount)
    assertTrue(fixture.touchPanDeltas.isEmpty())
  }

  @Test
  fun `common trackpad scale event precedes child consumption`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture, consumeIndirectInputAtChild = true)
    val initialZoom = fixture.zoomController.displayZoom

    onNodeWithTag(EditorTag).performTrackpadInput {
      updatePointerTo(Offset(x = 180f, y = 160f))
      scale(1.25f)
    }
    waitForIdle()

    assertEquals(initialZoom * 1.25f, fixture.zoomController.displayZoom, 0.0001f)
  }

  @Test
  fun `common trackpad scale event does not move navigation route`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)
    val initialZoom = fixture.zoomController.displayZoom

    onNodeWithTag(NavigationEditorTag).performTrackpadInput {
      updatePointerTo(Offset(x = 180f, y = 160f))
      scale(1.2f)
    }
    waitForIdle()

    assertTrue(fixture.zoomController.displayZoom > initialZoom)
    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `platform indirect scale bridge uses the editor interaction owner`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)
    val initialZoom = fixture.zoomController.displayZoom

    runOnIdle {
      assertTrue(fixture.platformIndirectScaleBridge.begin())
      assertTrue(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.25f,
        )
      )
      fixture.platformIndirectScaleBridge.end()
    }

    assertEquals(initialZoom * 1.25f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
  }

  @Test
  fun `platform indirect scale reports editor pointer input`() = runComposeUiTest {
    val fixture = Fixture()
    var pointerInputCount = 0
    setEditorContent(fixture = fixture, onEditorPointerInput = { pointerInputCount += 1 })

    runOnIdle {
      assertTrue(fixture.platformIndirectScaleBridge.begin())
      assertTrue(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.1f,
        )
      )
      fixture.platformIndirectScaleBridge.end()
    }

    assertEquals(1, pointerInputCount)
  }

  @Test
  fun `direct pointer down reports editor input once`() = runComposeUiTest {
    val fixture = Fixture()
    var pointerInputCount = 0
    setEditorContent(fixture = fixture, onEditorPointerInput = { pointerInputCount += 1 })

    onNodeWithTag(EditorTag).performTouchInput { down(center) }
    waitForIdle()

    assertEquals(1, pointerInputCount)

    onNodeWithTag(EditorTag).performTouchInput { up() }
  }

  @Test
  fun `platform indirect scale takes over a pending physical pan candidate`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)
    val initialZoom = fixture.zoomController.displayZoom

    onNodeWithTag(EditorTag).performTrackpadInput {
      moveTo(Offset(x = 100f, y = 100f))
      press()
    }
    waitForIdle()

    runOnIdle {
      assertTrue(fixture.platformIndirectScaleBridge.begin())
      assertTrue(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.1f,
        )
      )
      fixture.platformIndirectScaleBridge.end()
    }

    onNodeWithTag(EditorTag).performTrackpadInput { release() }
    waitForIdle()
    assertTrue(fixture.zoomController.displayZoom > initialZoom)
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertFalse(fixture.scrollGestureLockState.isLocked)
  }

  @Test
  fun `platform indirect scale takes over a pressed tap`() = runComposeUiTest {
    val fixture = Fixture(tapEligible = true)
    setEditorContent(fixture)
    val editor = onNodeWithTag(EditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performMouseInput {
      moveTo(center)
      press()
    }
    waitForIdle()

    runOnIdle {
      assertTrue(fixture.platformIndirectScaleBridge.begin())
      assertTrue(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.1f,
        )
      )
      fixture.platformIndirectScaleBridge.end()
    }

    assertTrue(fixture.host.longPressDispatchCancelCount > 0)
    assertTrue(fixture.zoomController.displayZoom > initialZoom)
    editor.performMouseInput { release() }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertFalse(fixture.scrollGestureLockState.isLocked)
  }

  @Test
  fun `late mouse press joins an active platform indirect scale without starting long press`() =
    runComposeUiTest {
      val fixture = Fixture(tapEligible = true)
      setEditorContent(fixture)
      val editor = onNodeWithTag(EditorTag)
      val initialZoom = fixture.zoomController.displayZoom

      runOnIdle { assertTrue(fixture.platformIndirectScaleBridge.begin()) }
      editor.performMouseInput {
        moveTo(center)
        press()
      }
      waitForIdle()

      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)
      assertEquals(0, fixture.host.longPressDispatchScheduleCount)
      runOnIdle {
        assertTrue(
          fixture.platformIndirectScaleBridge.update(
            focalInRootPx = Offset(x = 180f, y = 160f),
            scaleFactor = 1.1f,
          )
        )
        fixture.platformIndirectScaleBridge.end()
      }

      assertTrue(fixture.zoomController.displayZoom > initialZoom)
      editor.performMouseInput { release() }
      waitForIdle()
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
      assertFalse(fixture.scrollGestureLockState.isLocked)
    }

  @Test
  fun `platform indirect scale cannot take over an active physical pan`() = runComposeUiTest {
    val fixture = Fixture(tapEligible = true)
    setEditorContent(fixture)
    val editor = onNodeWithTag(EditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performMouseInput {
      moveTo(Offset(x = 100f, y = 100f))
      press()
      moveTo(Offset(x = 120f, y = 100f))
    }
    waitForIdle()
    assertEquals(EditorInteractionMode.Panning, fixture.controller.interactionMode)
    val panDeltaCountBeforeIndirectInput = fixture.touchPanDeltas.size

    editor.performKeyInput { keyDown(Key.MetaLeft) }
    editor.performMouseInput { scroll(Offset(x = 0f, y = -24f)) }
    editor.performKeyInput { keyUp(Key.MetaLeft) }
    editor.performTrackpadInput {
      updatePointerTo(Offset(x = 120f, y = 100f))
      pan(Offset(x = -40f, y = 60f))
    }
    assertEquals(EditorInteractionMode.Panning, fixture.controller.interactionMode)
    assertEquals(panDeltaCountBeforeIndirectInput, fixture.touchPanDeltas.size)
    runOnIdle { assertFalse(fixture.platformIndirectScaleBridge.begin()) }

    editor.performMouseInput { release() }
    waitForIdle()
    assertEquals(initialZoom, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertFalse(fixture.scrollGestureLockState.isLocked)
  }

  @Test
  fun `foreign physical sequence cancels active platform indirect scale`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture = fixture, editorWidth = 300.dp)

    runOnIdle {
      assertTrue(fixture.platformIndirectScaleBridge.begin())
      assertTrue(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.1f,
        )
      )
    }

    onNodeWithTag(RootTag).performTouchInput {
      down(pointerId = 0, position = Offset(x = 350f, y = 100f))
    }

    runOnIdle {
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
      assertFalse(
        fixture.platformIndirectScaleBridge.update(
          focalInRootPx = Offset(x = 180f, y = 160f),
          scaleFactor = 1.1f,
        )
      )
    }

    onNodeWithTag(RootTag).performTouchInput { up(pointerId = 0) }
  }

  @Test
  fun `common trackpad pan event scrolls without moving navigation`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTrackpadInput {
      updatePointerTo(Offset(x = 180f, y = 160f))
      pan(Offset(x = -40f, y = 60f))
    }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.isNotEmpty())
    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `common trackpad pan event takes over a pending physical candidate`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    setEditorContent(fixture)
    val editor = onNodeWithTag(EditorTag)

    editor.performMouseInput {
      moveTo(center)
      press()
    }
    waitForIdle()
    editor.performTrackpadInput {
      updatePointerTo(center)
      pan(Offset(x = -40f, y = 60f))
    }
    editor.performMouseInput { release() }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.isNotEmpty())
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertFalse(fixture.scrollGestureLockState.isLocked)
  }

  @Test
  fun `modified physical click drag remains pan and never zooms`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)
    val editor = onNodeWithTag(EditorTag)
    val initialZoom = fixture.zoomController.displayZoom

    editor.performKeyInput { keyDown(Key.MetaLeft) }
    editor.performTrackpadInput {
      moveTo(Offset(x = 100f, y = 100f))
      press()
      moveBy(Offset(x = 20f, y = 0f))
      release()
    }
    editor.performKeyInput { keyUp(Key.MetaLeft) }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.isNotEmpty())
    assertEquals(initialZoom, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(initialZoom, fixture.zoomController.renderZoom, 0.0001f)
    assertFalse(fixture.scrollGestureLockState.isLocked)
  }

  @Test
  fun `desktop trackpad click drag commits main editor back swipe`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTrackpadInput {
      moveTo(Offset(x = 80f, y = center.y))
      press()
      repeat(18) { moveBy(Offset(x = 10f, y = 0f), delayMillis = 1L) }
    }
    waitUntil { onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left > 100f }
    onNodeWithTag(NavigationEditorTag).performTrackpadInput { release() }

    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
    onAllNodes(hasTestTag(NavigationEditorTag)).assertCountEquals(0)
  }

  @Test
  fun `desktop editor scroll owns drag before full-area back swipe`() = runComposeUiTest {
    val fixture = Fixture()
    val navigator = Navigator(Route.Home)
    val touchSlop =
      setNavigationEditorContent(
        fixture = fixture,
        navigator = navigator,
        usePlatformTouchSlop = true,
      )
    val editor = onNodeWithTag(NavigationEditorTag)

    editor.performTrackpadInput {
      moveTo(Offset(x = 80f, y = center.y))
      press()
      moveBy(Offset(x = touchSlop - 1f, y = 0f), delayMillis = 100L)
    }
    waitForIdle()

    assertEquals(
      expected = 0f,
      actual = editor.fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )

    editor.performTrackpadInput { moveBy(Offset(x = touchSlop * 3f, y = 0f), delayMillis = 100L) }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.isNotEmpty())
    assertEquals(
      expected = 0f,
      actual = editor.fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )

    editor.performTrackpadInput { release() }
    waitForIdle()
    assertEquals(NavigationEditorRoute, navigator.current)
  }

  @Test
  fun `desktop nested back swipe moves once by activation overshoot`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    val touchSlop =
      setNavigationEditorContent(
        fixture = fixture,
        navigator = navigator,
        usePlatformTouchSlop = true,
      )
    val editor = onNodeWithTag(NavigationEditorTag)
    val initialDrag = touchSlop - 2f
    val followingDragCount = 6
    val followingDrag = 10f
    val totalDrag = initialDrag + followingDragCount * followingDrag

    editor.performTrackpadInput {
      moveTo(Offset(x = 80f, y = center.y))
      press()
      moveBy(Offset(x = initialDrag, y = 0f), delayMillis = 100L)
      repeat(followingDragCount) { moveBy(Offset(x = followingDrag, y = 0f), delayMillis = 100L) }
    }
    waitForIdle()

    assertEquals(
      expected = totalDrag - touchSlop * 3f,
      actual = editor.fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )

    editor.performTrackpadInput { release() }
    waitForIdle()
    assertEquals(NavigationEditorRoute, navigator.current)
  }

  @Test
  fun `desktop trackpad click drag ignores overlapping scroll signals`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performMouseInput {
      moveTo(Offset(x = 80f, y = center.y))
      press()
    }
    waitForIdle()
    repeat(18) {
      onNodeWithTag(NavigationEditorTag).performMouseInput {
        moveBy(Offset(x = 10f, y = 0f), delayMillis = 1L)
      }
      waitForIdle()
      onNodeWithTag(NavigationEditorTag).performMouseInput { scroll(Offset(x = -1f, y = 0f)) }
      waitForIdle()
    }
    onNodeWithTag(NavigationEditorTag).performMouseInput { release() }
    waitForIdle()

    assertEquals(Route.Home, navigator.current)
    onAllNodes(hasTestTag(NavigationEditorTag)).assertCountEquals(0)
  }

  @Test
  fun `slow desktop trackpad click drag returns editor to origin before halfway`() =
    runComposeUiTest {
      val fixture = Fixture(scrollConsumer = { Offset.Zero })
      val navigator = Navigator(Route.Home)
      setNavigationEditorContent(fixture = fixture, navigator = navigator)

      onNodeWithTag(NavigationEditorTag).performTrackpadInput {
        moveTo(Offset(x = 80f, y = center.y))
        press()
        repeat(12) { moveBy(Offset(x = 5f, y = 0f), delayMillis = 100L) }
      }
      waitUntil { onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left > 0.5f }
      onNodeWithTag(NavigationEditorTag).performTrackpadInput { release() }
      waitForIdle()

      assertEquals(NavigationEditorRoute, navigator.current)
      assertEquals(
        expected = 0f,
        actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
        absoluteTolerance = 0.5f,
      )
    }

  @Test
  fun `rapid desktop trackpad release completes committed back swipe`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTrackpadInput {
      moveTo(Offset(x = 80f, y = center.y))
      press()
      repeat(18) { moveBy(Offset(x = 10f, y = 0f), delayMillis = 1L) }
      release()
    }

    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
    onAllNodes(hasTestTag(NavigationEditorTag)).assertCountEquals(0)
  }

  @Test
  fun `pointer signal scroll does not move navigation route`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performMouseInput {
      moveTo(Offset(x = 80f, y = center.y))
      listOf(-18f, -9f, -4f, -2f, -1f).forEach { deltaX -> scroll(Offset(x = deltaX, y = 0f)) }
    }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.any { delta -> delta.x > 0f })
    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `pointer signal scroll takes over a pending pointer without moving navigation`() =
    runComposeUiTest {
      val fixture = Fixture(scrollConsumer = { Offset.Zero })
      val navigator = Navigator(Route.Home)
      setNavigationEditorContent(fixture = fixture, navigator = navigator)

      onNodeWithTag(NavigationEditorTag).performMouseInput {
        moveTo(Offset(x = 80f, y = center.y))
        press()
      }
      waitForIdle()
      listOf(-120f, -80f, -40f, -20f).forEach { deltaX ->
        onNodeWithTag(NavigationEditorTag).performMouseInput { scroll(Offset(x = deltaX, y = 0f)) }
        waitForIdle()
      }
      onNodeWithTag(NavigationEditorTag).performMouseInput { release() }
      waitForIdle()

      assertTrue(fixture.touchPanDeltas.any { delta -> delta.x > 0f })
      assertEquals(NavigationEditorRoute, navigator.current)
      assertEquals(
        expected = 0f,
        actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
        absoluteTolerance = 0.5f,
      )
      assertFalse(fixture.scrollGestureLockState.isLocked)
    }

  @Test
  fun `accessibility scroll does not move navigation route`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performSemanticsAction(SemanticsActions.ScrollBy) { action ->
      assertTrue(action(12f, 0f))
    }
    waitForIdle()

    assertTrue(fixture.touchPanDeltas.any { delta -> delta.x > 0f })
    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `accessibility scroll remains navigation inert while a pointer is pressed`() =
    runComposeUiTest {
      val fixture = Fixture(scrollConsumer = { Offset.Zero })
      val navigator = Navigator(Route.Home)
      setNavigationEditorContent(fixture = fixture, navigator = navigator)

      onNodeWithTag(NavigationEditorTag).performMouseInput {
        moveTo(Offset(x = 80f, y = center.y))
        press()
      }
      waitForIdle()
      onNodeWithTag(NavigationEditorTag).performSemanticsAction(SemanticsActions.ScrollBy) { action
        ->
        assertTrue(action(120f, 0f))
      }
      waitForIdle()
      onNodeWithTag(NavigationEditorTag).performMouseInput { release() }
      waitForIdle()

      assertTrue(fixture.touchPanDeltas.any { delta -> delta.x > 0f })
      assertEquals(NavigationEditorRoute, navigator.current)
      assertEquals(
        expected = 0f,
        actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
        absoluteTolerance = 0.5f,
      )
    }

  @Test
  fun `interaction boundary preserves accessibility scroll by action`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performSemanticsAction(SemanticsActions.ScrollBy) { action ->
      assertTrue(action(12f, 24f))
    }
    waitForIdle()
    assertEquals(Offset(x = 12f, y = 24f), fixture.touchPanDeltas.fold(Offset.Zero, Offset::plus))
  }

  @Test
  fun `interaction boundary preserves accessibility scroll by offset action`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    coroutineScope {
      val scrollByOffset =
        onNodeWithTag(EditorTag).fetchSemanticsNode().config[SemanticsActions.ScrollByOffset]
      var consumedOffset = Offset.Zero
      val scrollJob = launch { consumedOffset = scrollByOffset(Offset(x = 8f, y = 16f)) }
      val deadline = mainClock.currentTime + 10_000L
      while (!scrollJob.isCompleted && mainClock.currentTime < deadline) {
        mainClock.advanceTimeByFrame()
        yield()
      }
      assertTrue(scrollJob.isCompleted)
      assertEquals(Offset(x = 8f, y = 16f), consumedOffset)
      assertEquals(Offset(x = 8f, y = 16f), fixture.touchPanDeltas.fold(Offset.Zero, Offset::plus))
    }
  }

  @Test
  fun `rapid main editor viewport release completes committed back swipe`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(Offset(x = 80f, y = center.y))
      repeat(18) { moveBy(Offset(x = 10f, y = 0f), delayMillis = 1L) }
      moveBy(Offset(x = 1f, y = 0f), delayMillis = 1L)
      up()
    }

    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
  }

  @Test
  fun `rapid main editor viewport release commits before halfway`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(Offset(x = 80f, y = center.y))
      repeat(12) { moveBy(Offset(x = 5f, y = 0f), delayMillis = 1L) }
      moveBy(Offset(x = 1f, y = 0f), delayMillis = 1L)
      up()
    }
    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
  }

  @Test
  fun `rapid edge release completes committed back swipe`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(Offset(x = 10f, y = center.y))
      repeat(18) { moveBy(Offset(x = 10f, y = 0f), delayMillis = 1L) }
      moveBy(Offset(x = 1f, y = 0f), delayMillis = 1L)
      up()
    }

    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
  }

  @Test
  fun `second pointer rolls active main editor back swipe to origin`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(x = 80f, y = center.y))
      moveBy(pointerId = 0, delta = Offset(x = 100f, y = 0f), delayMillis = 100L)
      moveBy(pointerId = 0, delta = Offset(x = 10f, y = 0f), delayMillis = 100L)
    }
    waitUntil { onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left > 0.5f }

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(pointerId = 1, position = Offset(x = 240f, y = center.y))
      up(pointerId = 1)
      up(pointerId = 0)
    }
    waitForIdle()

    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `second pointer rolls active edge back swipe to origin`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    setNavigationEditorContent(fixture = fixture, navigator = navigator)

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(x = 10f, y = center.y))
      moveBy(pointerId = 0, delta = Offset(x = 100f, y = 0f), delayMillis = 100L)
      moveBy(pointerId = 0, delta = Offset(x = 10f, y = 0f), delayMillis = 100L)
    }
    waitUntil { onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left > 0.5f }

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(pointerId = 1, position = Offset(x = 240f, y = center.y))
      up(pointerId = 1)
      up(pointerId = 0)
    }
    waitForIdle()

    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `cancelling main editor viewport pan rolls back active back swipe`() = runComposeUiTest {
    val fixture = Fixture(scrollConsumer = { Offset.Zero })
    val navigator = Navigator(Route.Home)
    val interactionEnabled = mutableStateOf(true)
    setNavigationEditorContent(
      fixture = fixture,
      navigator = navigator,
      interactionEnabled = { interactionEnabled.value },
    )

    onNodeWithTag(NavigationEditorTag).performTouchInput {
      down(center)
      moveBy(Offset(x = 100f, y = 0f))
      moveBy(Offset(x = 1f, y = 0f))
    }
    runOnIdle { interactionEnabled.value = false }
    waitForIdle()

    onNodeWithTag(NavigationEditorTag).performTouchInput { up() }
    waitForIdle()

    assertEquals(NavigationEditorRoute, navigator.current)
    assertEquals(
      expected = 0f,
      actual = onNodeWithTag(NavigationEditorTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun `released viewport pan completes nested terminal after node scope cancellation`() =
    runComposeUiTest {
      lateinit var nodeJob: Job
      var preFlingCount = 0
      var postFlingCount = 0
      lateinit var driver: EditorViewportScrollDriver

      setContent {
        val scope = rememberCoroutineScope()
        nodeJob = remember { Job() }
        val nodeScope = remember { CoroutineScope(scope.coroutineContext + nodeJob) }
        val dispatcher = remember { NestedScrollDispatcher() }
        val state = remember { Scrollable2DState { delta -> delta } }
        val parentConnection = remember {
          object : NestedScrollConnection {
            override suspend fun onPreFling(available: Velocity): Velocity {
              preFlingCount += 1
              return Velocity.Zero
            }

            override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
              postFlingCount += 1
              return available
            }
          }
        }
        driver = remember {
          EditorViewportScrollDriver(
            scrollableState = { state },
            nestedScrollDispatcher = { dispatcher },
            flingBehavior = {
              object : FlingBehavior {
                override suspend fun ScrollScope.performFling(initialVelocity: Float): Float =
                  error("Detached viewport must not run its own fling")
              }
            },
            touchSlopProvider = { 0f },
            maximumFlingVelocityProvider = { Float.MAX_VALUE },
            launch = { block -> nodeScope.launch { block() } },
          )
        }
        Box(
          Modifier.size(100.dp)
            .nestedScroll(parentConnection)
            .nestedScroll(NoOpNestedScrollConnection, dispatcher)
        )
      }
      waitForIdle()

      runOnIdle {
        assertTrue(driver.start())
        driver.update(Offset(x = 20f, y = 0f))
        driver.end(Velocity(x = 1_000f, y = 0f))
        nodeJob.cancel()
      }

      waitUntil(timeoutMillis = 1_000L) { preFlingCount == 1 && postFlingCount == 1 }
    }

  @Test
  fun `pinch routing samples once per pointer event frame`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
    }
    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 1, position = Offset(200f, 100f))
    }
    assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

    val revisionBeforeMove = fixture.viewportState.lastScrollRevision
    onNodeWithTag(EditorTag).performTouchInput {
      updatePointerTo(pointerId = 0, position = Offset(75f, 100f))
      updatePointerTo(pointerId = 1, position = Offset(225f, 100f))
      move()
    }
    assertEquals(revisionBeforeMove + 1, fixture.viewportState.lastScrollRevision)

    onNodeWithTag(EditorTag).performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }
  }

  @Test
  fun `page and extension area pointers start one pinch`() = runComposeUiTest {
    val fixture =
      Fixture(
        editorBoundsInRoot = Rect(left = 100f, top = 0f, right = 300f, bottom = 400f),
        pageSize = PageSize(width = 200f, height = 400f),
      )
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(x = 150f, y = 100f))
      down(pointerId = 1, position = Offset(x = 350f, y = 100f))
    }

    assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

    onNodeWithTag(EditorTag).performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }
  }

  @Test
  fun `third pinch pointer cancels and suppresses restart until all pointers are up`() =
    runComposeUiTest {
      val fixture = Fixture()
      setEditorContent(fixture)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
      }
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 1, position = Offset(200f, 100f))
      }
      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 2, position = Offset(150f, 200f))
      }
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 2) }
      onNodeWithTag(EditorTag).performTouchInput {
        updatePointerTo(pointerId = 0, position = Offset(70f, 100f))
        updatePointerTo(pointerId = 1, position = Offset(230f, 100f))
        move()
      }
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
      }
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 1, position = Offset(200f, 100f))
      }
      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    }

  @Test
  fun `surviving pinch pointer resumes normal nested pan without touch slop`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
      down(pointerId = 1, position = Offset(200f, 100f))
      updatePointerTo(pointerId = 0, position = Offset(50f, 100f))
      updatePointerTo(pointerId = 1, position = Offset(250f, 100f))
      move()
      up(pointerId = 1)
    }
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertTrue(fixture.touchPanDeltas.isEmpty())

    onNodeWithTag(EditorTag).performTouchInput {
      moveTo(pointerId = 0, position = Offset(50f, 101f))
      moveTo(pointerId = 0, position = Offset(50f, 102f))
    }
    waitForIdle()

    assertEquals(EditorInteractionMode.Panning, fixture.controller.interactionMode)
    assertEquals(listOf(Offset(x = 0f, y = 1f), Offset(x = 0f, y = 1f)), fixture.touchPanDeltas)
    assertTrue(fixture.nestedScrollAvailable.isNotEmpty())

    onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 0) }
    waitForIdle()
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertTrue(fixture.flingVelocities.isNotEmpty())
    assertTrue(fixture.flingVelocities.all { velocity -> velocity.y < 1_000f })
  }

  @Test
  fun `detaching the interaction boundary cancels active pinch`() = runComposeUiTest {
    val fixture = Fixture()
    val includeInteractionBoundary = mutableStateOf(true)
    setEditorContent(
      fixture = fixture,
      includeInteractionBoundary = { includeInteractionBoundary.value },
    )

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
      down(pointerId = 1, position = Offset(200f, 100f))
    }
    assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

    runOnIdle { includeInteractionBoundary.value = false }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

    onNodeWithTag(EditorTag).performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
  }

  @Test
  fun `detaching the interaction boundary cancels active pan`() = runComposeUiTest {
    val fixture = Fixture()
    val includeInteractionBoundary = mutableStateOf(true)
    setEditorContent(
      fixture = fixture,
      includeInteractionBoundary = { includeInteractionBoundary.value },
    )

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
      moveTo(pointerId = 0, position = Offset(120f, 100f))
    }
    waitForIdle()
    assertEquals(EditorInteractionMode.Panning, fixture.controller.interactionMode)

    runOnIdle { includeInteractionBoundary.value = false }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

    onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 0) }
    waitForIdle()
    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
  }

  @Test
  fun `fresh viewport pan waits touch slop and preserves fling direction`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
      updatePointerTo(pointerId = 0, position = Offset(104f, 100f))
      move()
    }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertTrue(fixture.touchPanDeltas.isEmpty())

    onNodeWithTag(EditorTag).performTouchInput {
      updatePointerTo(pointerId = 0, position = Offset(112f, 100f))
      move()
    }
    waitForIdle()

    assertEquals(EditorInteractionMode.Panning, fixture.controller.interactionMode)
    assertEquals(Offset(x = 4f, y = 0f), fixture.touchPanDeltas.single())

    onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 0) }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertTrue(fixture.flingPanDeltas.isNotEmpty())
    assertTrue(
      fixture.flingPanDeltas.all { delta -> delta.x > 0f && abs(delta.y) < 0.001f },
      "Expected a horizontal positive fling, got ${fixture.flingPanDeltas}",
    )
  }

  @Test
  fun `stationary editor touch catches active self fling without becoming pan`() =
    runComposeUiTest {
      var flingStarted = false
      var flingCancelled = false
      val releaseFling = CompletableDeferred<Unit>()
      val blockingFling =
        object : FlingBehavior {
          override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
            flingStarted = true
            try {
              releaseFling.await()
            } catch (cancellation: CancellationException) {
              flingCancelled = true
              throw cancellation
            }
            return 0f
          }
        }
      val fixture = Fixture(flingBehaviorOverride = blockingFling)
      setEditorContent(fixture)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
        moveTo(pointerId = 0, position = Offset(120f, 100f))
        moveTo(pointerId = 0, position = Offset(140f, 100f))
        up(pointerId = 0)
      }
      waitUntil { flingStarted }
      val touchDeltaCountBeforeInterruption = fixture.touchPanDeltas.size
      val pointerCancelCountBeforeInterruption = fixture.host.pointerStreamCancelCount

      try {
        onNodeWithTag(EditorTag).performTouchInput {
          down(pointerId = 0, position = Offset(100f, 100f))
        }

        waitUntil(timeoutMillis = 1_000L) {
          flingCancelled && fixture.nestedPostFlingAvailable.size == 1
        }
        assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
        assertEquals(1, fixture.nestedPreFlingAvailable.size)
        assertEquals(1, fixture.nestedPostFlingAvailable.size)
        assertEquals(pointerCancelCountBeforeInterruption, fixture.host.pointerStreamCancelCount)

        onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 0) }
        waitForIdle()

        assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
        assertEquals(touchDeltaCountBeforeInterruption, fixture.touchPanDeltas.size)
        assertEquals(1, fixture.nestedPreFlingAvailable.size)
        assertEquals(1, fixture.nestedPostFlingAvailable.size)
      } finally {
        releaseFling.complete(Unit)
      }
    }

  @Test
  fun `stationary editor touch catches programmatic scroll without becoming pan`() =
    runComposeUiTest {
      var scrollStarted = false
      var scrollCancelled = false
      val releaseScroll = CompletableDeferred<Unit>()
      val fixture = Fixture()
      lateinit var compositionScope: CoroutineScope
      setEditorContent(fixture, onCoroutineScope = { compositionScope = it })
      val scrollJob =
        compositionScope.launch(start = CoroutineStart.UNDISPATCHED) {
          try {
            fixture.scrollableState.scroll(MutatePriority.Default) {
              scrollStarted = true
              releaseScroll.await()
            }
          } catch (_: CancellationException) {
            scrollCancelled = true
          }
        }
      waitUntil { scrollStarted }
      assertTrue(fixture.scrollableState.isScrollInProgress)

      try {
        onNodeWithTag(EditorTag).performTouchInput {
          down(pointerId = 0, position = Offset(100f, 100f))
        }
        waitUntil(timeoutMillis = 1_000L) { scrollCancelled }

        assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
        assertTrue(fixture.touchPanDeltas.isEmpty())
        assertTrue(fixture.nestedPreFlingAvailable.isEmpty())
        assertTrue(fixture.nestedPostFlingAvailable.isEmpty())

        onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 0) }
        waitForIdle()

        assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
        assertTrue(fixture.nestedPreFlingAvailable.isEmpty())
        assertTrue(fixture.nestedPostFlingAvailable.isEmpty())
      } finally {
        releaseScroll.complete(Unit)
        scrollJob.join()
      }
    }

  @Test
  fun `movement after catching self fling becomes pan without fresh touch slop`() =
    runComposeUiTest {
      var flingInvocation = 0
      var firstFlingStarted = false
      var firstFlingCancelled = false
      val releaseFirstFling = CompletableDeferred<Unit>()
      val blockingFirstFling =
        object : FlingBehavior {
          override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
            flingInvocation += 1
            if (flingInvocation == 1) {
              firstFlingStarted = true
              try {
                releaseFirstFling.await()
              } catch (cancellation: CancellationException) {
                firstFlingCancelled = true
                throw cancellation
              }
            }
            return 0f
          }
        }
      val fixture = Fixture(flingBehaviorOverride = blockingFirstFling)
      setEditorContent(fixture)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
        moveTo(pointerId = 0, position = Offset(120f, 100f))
        moveTo(pointerId = 0, position = Offset(140f, 100f))
        up(pointerId = 0)
      }
      waitUntil { firstFlingStarted }
      val touchDeltaCountBeforeInterruption = fixture.touchPanDeltas.size

      try {
        onNodeWithTag(EditorTag).performTouchInput {
          down(pointerId = 0, position = Offset(100f, 100f))
        }
        waitUntil(timeoutMillis = 1_000L) {
          firstFlingCancelled && fixture.nestedPostFlingAvailable.size == 1
        }
        assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

        onNodeWithTag(EditorTag).performTouchInput {
          moveTo(pointerId = 0, position = Offset(101f, 100f))
        }
        waitUntil { fixture.touchPanDeltas.size > touchDeltaCountBeforeInterruption }

        assertEquals(EditorInteractionMode.Panning, fixture.controller.interactionMode)
        assertEquals(Offset(x = 1f, y = 0f), fixture.touchPanDeltas.last())

        onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 0) }
        waitUntil {
          fixture.controller.interactionMode == EditorInteractionMode.Idle &&
            fixture.nestedPostFlingAvailable.size == 2
        }
        assertEquals(2, fixture.nestedPreFlingAvailable.size)
      } finally {
        releaseFirstFling.complete(Unit)
      }
    }

  @Test
  fun `second pointer after catching self fling starts pinch`() = runComposeUiTest {
    var flingStarted = false
    var flingCancelled = false
    val releaseFling = CompletableDeferred<Unit>()
    val blockingFling =
      object : FlingBehavior {
        override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
          flingStarted = true
          try {
            releaseFling.await()
          } catch (cancellation: CancellationException) {
            flingCancelled = true
            throw cancellation
          }
          return 0f
        }
      }
    val fixture = Fixture(flingBehaviorOverride = blockingFling)
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
      moveTo(pointerId = 0, position = Offset(120f, 100f))
      moveTo(pointerId = 0, position = Offset(140f, 100f))
      up(pointerId = 0)
    }
    waitUntil { flingStarted }

    try {
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
      }
      waitUntil(timeoutMillis = 1_000L) {
        flingCancelled && fixture.nestedPostFlingAvailable.size == 1
      }
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 1, position = Offset(200f, 100f))
      }
      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    } finally {
      releaseFling.complete(Unit)
    }
  }

  @Test
  fun `editor and foreign control pointers cancel editor interaction until all up`() =
    runComposeUiTest {
      val fixture = Fixture()
      setEditorContent(fixture, editorWidth = 300.dp)

      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
        down(pointerId = 1, position = Offset(350f, 100f))
      }

      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

      onNodeWithTag(RootTag).performTouchInput {
        updatePointerTo(pointerId = 0, position = Offset(75f, 100f))
        updatePointerTo(pointerId = 1, position = Offset(375f, 100f))
        move()
        up(pointerId = 1)
        up(pointerId = 0)
      }

      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
        down(pointerId = 1, position = Offset(200f, 100f))
      }
      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

      onNodeWithTag(RootTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    }

  @Test
  fun `foreign pointer already down prevents editor pointer stream from starting`() =
    runComposeUiTest {
      val fixture = Fixture()
      setEditorContent(fixture = fixture, editorWidth = 300.dp)

      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 0, position = Offset(350f, 100f))
      }
      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 1, position = Offset(100f, 100f))
      }

      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
      assertEquals(0, fixture.host.pointerStreamCancelCount)
      assertTrue(fixture.touchPanDeltas.isEmpty())

      onNodeWithTag(RootTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    }

  @Test
  fun `fully foreign multi-touch stays outside editor gesture ownership`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture = fixture, editorWidth = 300.dp)

    onNodeWithTag(RootTag).performTouchInput {
      down(pointerId = 0, position = Offset(320f, 100f))
      down(pointerId = 1, position = Offset(360f, 100f))
      updatePointerTo(pointerId = 0, position = Offset(315f, 100f))
      updatePointerTo(pointerId = 1, position = Offset(365f, 100f))
      move()
      up(pointerId = 1)
      up(pointerId = 0)
    }
    waitForIdle()

    assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
    assertEquals(0, fixture.host.pointerStreamCancelCount)
    assertTrue(fixture.touchPanDeltas.isEmpty())
  }

  @Test
  fun `active pinch keeps tracking raw positions outside start bounds`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture, editorWidth = 300.dp)

    onNodeWithTag(RootTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
      down(pointerId = 1, position = Offset(200f, 100f))
    }
    val revisionBeforeMove = fixture.viewportState.lastScrollRevision

    onNodeWithTag(RootTag).performTouchInput {
      updatePointerTo(pointerId = 0, position = Offset(50f, 100f))
      updatePointerTo(pointerId = 1, position = Offset(350f, 100f))
      move()
    }

    assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)
    assertEquals(revisionBeforeMove + 1, fixture.viewportState.lastScrollRevision)

    onNodeWithTag(RootTag).performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setEditorContent(
    fixture: Fixture,
    includeInteractionBoundary: () -> Boolean = { true },
    editorWidth: androidx.compose.ui.unit.Dp = 400.dp,
    consumeIndirectInputAtChild: Boolean = false,
    onEditorPointerInput: () -> Unit = {},
    onCoroutineScope: (CoroutineScope) -> Unit = {},
  ) {
    setContent {
      val nestedScrollDispatcher = remember { NestedScrollDispatcher() }
      val screenPointerSequence = remember { EditorScreenPointerSequence() }
      onCoroutineScope(rememberCoroutineScope())
      Box(
        Modifier.size(400.dp)
          .testTag(RootTag)
          .observeEditorScreenPointerSequence(screenPointerSequence)
      ) {
        Box(
          Modifier.size(width = editorWidth, height = 400.dp)
            .testTag(EditorTag)
            .nestedScroll(fixture.nestedScrollConnection)
            .nestedScroll(NoOpNestedScrollConnection, nestedScrollDispatcher)
            .then(
              if (includeInteractionBoundary()) {
                Modifier.editorInteractions(
                  interactionController = fixture.controller,
                  geometry = fixture.host,
                  screenPointerSequence = screenPointerSequence,
                  platformIndirectScaleBridge = fixture.platformIndirectScaleBridge,
                  scrollGestureLockState = fixture.scrollGestureLockState,
                  scrollableState = fixture.scrollableState,
                  nestedScrollDispatcher = nestedScrollDispatcher,
                  flingBehavior = fixture.flingBehavior,
                  touchSlop = 8f,
                  density = 1f,
                  onEditorPointerInput = onEditorPointerInput,
                )
              } else {
                Modifier
              }
            )
        ) {
          if (consumeIndirectInputAtChild) {
            Box(
              Modifier.fillMaxSize().pointerInput(Unit) {
                awaitPointerEventScope {
                  while (true) {
                    val event = awaitPointerEvent(PointerEventPass.Main)
                    if (
                      event.type == PointerEventType.Scroll ||
                        event.type == PointerEventType.ScaleStart ||
                        event.type == PointerEventType.ScaleChange ||
                        event.type == PointerEventType.ScaleEnd
                    ) {
                      event.changes.forEach { change -> change.consume() }
                    }
                  }
                }
              }
            )
          }
        }
        if (editorWidth < 400.dp) {
          Box(
            Modifier.align(androidx.compose.ui.Alignment.TopEnd).size(400.dp - editorWidth, 400.dp)
          )
        }
      }
    }
    waitForIdle()
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setNavigationEditorContent(
    fixture: Fixture,
    navigator: Navigator,
    interactionEnabled: () -> Boolean = { true },
    usePlatformTouchSlop: Boolean = false,
  ): Float {
    var effectiveTouchSlop = 0f
    setContent {
      effectiveTouchSlop =
        if (usePlatformTouchSlop) LocalViewConfiguration.current.touchSlop else 8f
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        if (route == NavigationEditorRoute) {
          val nestedScrollDispatcher = remember { NestedScrollDispatcher() }
          val navigationPopNestedScroll = LocalNavigationPopNestedScroll.current
          val screenPointerSequence = remember { EditorScreenPointerSequence() }
          Box(Modifier.fillMaxSize().observeEditorScreenPointerSequence(screenPointerSequence)) {
            Box(
              Modifier.fillMaxSize()
                .testTag(NavigationEditorTag)
                .navigationPopNestedScroll()
                .nestedScroll(NoOpNestedScrollConnection, nestedScrollDispatcher)
                .editorInteractions(
                  interactionController = fixture.controller,
                  geometry = fixture.host,
                  screenPointerSequence = screenPointerSequence,
                  platformIndirectScaleBridge = fixture.platformIndirectScaleBridge,
                  scrollGestureLockState = fixture.scrollGestureLockState,
                  scrollableState = fixture.scrollableState,
                  nestedScrollDispatcher = nestedScrollDispatcher,
                  flingBehavior = fixture.flingBehavior,
                  touchSlop = effectiveTouchSlop,
                  density = 1f,
                  enabled = interactionEnabled(),
                  onNestedScrollCancel = { navigationPopNestedScroll?.cancel() },
                )
            )
          }
        } else {
          Box(Modifier.fillMaxSize())
        }
      }
      LaunchedEffect(Unit) { navigator.navigate(NavigationEditorRoute) }
    }
    waitUntil { navigator.current == NavigationEditorRoute && !navigator.isTransitioning }
    return effectiveTouchSlop
  }

  private class Fixture(
    private val scrollConsumer: (Offset) -> Offset = { delta -> delta },
    editorBoundsInRoot: Rect = Rect(left = 0f, top = 0f, right = 400f, bottom = 400f),
    pageSize: PageSize = PageSize(width = 720f, height = 960f),
    flingBehaviorOverride: FlingBehavior? = null,
    tapEligible: Boolean = false,
  ) {
    val touchPanDeltas = mutableListOf<Offset>()
    val flingPanDeltas = mutableListOf<Offset>()
    val nestedScrollAvailable = mutableListOf<Offset>()
    val nestedPreFlingAvailable = mutableListOf<Velocity>()
    val nestedPostFlingAvailable = mutableListOf<Velocity>()
    val flingVelocities = mutableListOf<Velocity>()
    private var recordingFling = false
    val scrollableState = Scrollable2DState { delta ->
      if (recordingFling) {
        flingPanDeltas += delta
      } else {
        touchPanDeltas += delta
      }
      scrollConsumer(delta)
    }
    val nestedScrollConnection =
      object : NestedScrollConnection {
        override fun onPostScroll(
          consumed: Offset,
          available: Offset,
          source: NestedScrollSource,
        ): Offset {
          nestedScrollAvailable += available
          return Offset.Zero
        }

        override suspend fun onPreFling(available: Velocity): Velocity {
          nestedPreFlingAvailable += available
          return Velocity.Zero
        }

        override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
          nestedPostFlingAvailable += available
          return Velocity.Zero
        }
      }
    val flingBehavior =
      flingBehaviorOverride
        ?: object : FlingBehavior {
          override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
            flingVelocities += Velocity(x = 0f, y = initialVelocity)
            recordingFling = true
            try {
              scrollBy(10f)
            } finally {
              recordingFling = false
            }
            return 0f
          }
        }
    val viewportState =
      EditorViewportState().apply {
        updateMeasuredBounds(
          viewportSize = Size(width = 400f, height = 400f),
          contentSize = Size(width = 2000f, height = 2000f),
        )
      }
    private val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = pageSize.width,
        pageHeight = pageSize.height,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    private val pageSizes = listOf(pageSize)
    val zoomController = EditorZoomController()
    val platformIndirectScaleBridge = EditorPlatformIndirectScaleBridge()
    val scrollGestureLockState = ScrollGestureLockState()
    private val uiState =
      EditorUiState().apply {
        updateDisplayZoom(1f)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateEditorBounds(boundsInRoot = editorBoundsInRoot, density = 1f)
      }
    val host = TestHost(tapEligible = tapEligible)
    private val editor = Editor(FakeFfiEditor(), CoroutineScope(Job()), Dispatchers.Unconfined)
    private val semantics =
      EditorInteractionSemantics(effects = host).apply {
        zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = 400f)
        viewportZoom.configure(
          EditorViewportZoomSemanticConfig(
            layoutSpec = layoutSpec,
            zoomController = zoomController,
            viewportState = viewportState,
            uiState = uiState,
            pageSizes = pageSizes,
            viewportWidth = 400f,
            density = 1f,
            onZoomSnap = {},
          )
        )
      }
    val controller =
      EditorInteractionController(
        editorProvider = { editor },
        effects = host,
        geometry = host,
        semantics = semantics,
        uiStateProvider = { uiState },
      )
  }

  private class TestHost(private val tapEligible: Boolean) :
    EditorInteractionEffects, EditorInteractionGeometry {
    var pointerStreamCancelCount = 0
    var tapDispatchScheduleCount = 0
    var longPressDispatchScheduleCount = 0
    var longPressDispatchCancelCount = 0

    override val density: Float = 1f

    override fun resolveInteractionPosition(positionInSurface: Offset): Offset? =
      positionInSurface.takeIf {
        tapEligible
      }

    override fun isTapEligible(positionInSurface: Offset): Boolean = tapEligible

    override fun resolvePoint(positionInNode: Offset): PagePoint? = null

    override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? = null

    override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? = null

    override fun dispatchEdgeAutoScroll(delta: Offset): Offset = Offset.Zero

    override fun scheduleTapDispatch(dispatchAtMillis: Long) {
      tapDispatchScheduleCount += 1
    }

    override fun cancelTapDispatch() {
      pointerStreamCancelCount += 1
    }

    override fun scheduleLongPressDispatch(
      pointerId: Long,
      position: Offset,
      dispatchAtMillis: Long,
    ) {
      longPressDispatchScheduleCount += 1
    }

    override fun cancelLongPressDispatch() {
      longPressDispatchCancelCount += 1
    }

    override fun launchInteraction(block: suspend () -> Unit) = Unit

    override fun requestFocus(editor: Editor): Boolean = false

    override fun requestSoftwareKeyboard() = Unit

    override fun enqueuePointerCancel() = Unit

    override fun setScrollGestureLocked(locked: Boolean) = Unit

    override fun performSelectionHaptic() = Unit

    override fun requestCurrentSelectionHead(version: Long) = Unit
  }

  private companion object {
    const val EditorTag = "editor-interactions"
    const val RootTag = "editor-interactions-root"
    const val NavigationEditorTag = "navigation-editor-route"
    val NavigationEditorRoute = Route.Editor("document-id")
    val NoOpNestedScrollConnection = object : NestedScrollConnection {}
  }
}
