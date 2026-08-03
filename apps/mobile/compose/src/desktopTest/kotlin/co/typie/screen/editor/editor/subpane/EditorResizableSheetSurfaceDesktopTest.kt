package co.typie.screen.editor.editor.subpane

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipeWithVelocity
import androidx.compose.ui.unit.dp
import co.typie.ext.ScrollGestureLockScope
import co.typie.ext.verticalScroll
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.HazeState
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorResizableSheetSurfaceDesktopTest {
  @Test
  fun editorOwnedKeyboardCapsRenderedHeightWithoutChangingPreferredHeight() = runComposeUiTest {
    var editorFocused by mutableStateOf(false)
    var geometry: EditorResizableSheetGeometry? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 180.dp,
            safeBottomInset = 0.dp,
            editorFocused = editorFocused,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            Box(Modifier.fillMaxSize())
          }
        }
      }
    }
    waitForIdle()

    assertEquals(360f, checkNotNull(geometry).sheetHeight)

    mainClock.autoAdvance = false
    runOnIdle { editorFocused = true }
    repeat(3) { mainClock.advanceTimeByFrame() }

    val limitingHeight = checkNotNull(geometry).sheetHeight
    assertTrue(
      limitingHeight in 180f..360f && limitingHeight != 180f && limitingHeight != 360f,
      "expected a limiting transition height, but was $limitingHeight",
    )

    mainClock.autoAdvance = true
    waitForIdle()
    assertEquals(180f, checkNotNull(geometry).sheetHeight)

    mainClock.autoAdvance = false
    runOnIdle { editorFocused = false }
    repeat(3) { mainClock.advanceTimeByFrame() }

    val restoringHeight = checkNotNull(geometry).sheetHeight
    assertTrue(
      restoringHeight in 180f..360f && restoringHeight != 180f && restoringHeight != 360f,
      "expected a restoring transition height, but was $restoringHeight",
    )

    mainClock.autoAdvance = true
    waitForIdle()

    assertEquals(360f, checkNotNull(geometry).sheetHeight)
  }

  @Test
  fun activeKeyboardCapRetargetsFromCurrentMotionToLatestInset() = runComposeUiTest {
    var editorFocused by mutableStateOf(false)
    var trustedImeBottomInset by mutableStateOf(180.dp)
    var geometry: EditorResizableSheetGeometry? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = trustedImeBottomInset,
            safeBottomInset = 0.dp,
            editorFocused = editorFocused,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            Box(Modifier.fillMaxSize())
          }
        }
      }
    }
    waitForIdle()

    mainClock.autoAdvance = false
    runOnIdle { editorFocused = true }
    repeat(3) { mainClock.advanceTimeByFrame() }
    val heightBeforeLastPreRetargetFrame = checkNotNull(geometry).sheetHeight
    mainClock.advanceTimeByFrame()
    val heightImmediatelyBeforeRetarget = checkNotNull(geometry).sheetHeight
    val preRetargetDelta = heightImmediatelyBeforeRetarget - heightBeforeLastPreRetargetFrame
    assertTrue(preRetargetDelta < 0f)

    runOnIdle { trustedImeBottomInset = 220.dp }
    mainClock.advanceTimeByFrame()
    val heightImmediatelyAfterRetarget = checkNotNull(geometry).sheetHeight
    val postRetargetDelta = heightImmediatelyAfterRetarget - heightImmediatelyBeforeRetarget

    assertTrue(postRetargetDelta <= 0f)
    assertTrue(abs(postRetargetDelta) >= abs(preRetargetDelta) * 0.25f)

    mainClock.autoAdvance = true
    waitForIdle()
    assertEquals(220f, checkNotNull(geometry).sheetHeight)
  }

  @Test
  fun foregroundOcclusionKeepsHeaderRevealedAcrossPanelToKeyboardHandoff() = runComposeUiTest {
    var foregroundOcclusion by mutableStateOf(EditorSubPaneForegroundOcclusion(height = 0.dp))
    var geometry: EditorResizableSheetGeometry? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 300.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 0.dp,
            safeBottomInset = 0.dp,
            editorFocused = true,
            foregroundOcclusion = foregroundOcclusion,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            Box(Modifier.fillMaxSize())
          }
        }
      }
    }
    waitForIdle()
    assertEquals(300f, checkNotNull(geometry).sheetHeight)

    runOnIdle {
      foregroundOcclusion =
        EditorSubPaneForegroundOcclusion(
          height = 330.dp,
          headerRevealHeight = EditorSubPaneHeaderRevealHeight,
        )
    }
    waitForIdle()
    assertEquals(398f, checkNotNull(geometry).sheetHeight)

    runOnIdle { foregroundOcclusion = foregroundOcclusion.copy(height = 280.dp) }
    waitForIdle()
    assertEquals(348f, checkNotNull(geometry).sheetHeight)

    runOnIdle { foregroundOcclusion = EditorSubPaneForegroundOcclusion(height = 0.dp) }
    waitForIdle()
    assertEquals(300f, checkNotNull(geometry).sheetHeight)
  }

  @Test
  fun tappingBottomPanelRevealFocusesSheetAndRestoresRememberedHeight() = runComposeUiTest {
    var geometry: EditorResizableSheetGeometry? = null
    var exposedKeyboardOcclusion = (-1).dp

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 600.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 320.dp,
            safeBottomInset = 0.dp,
            editorFocused = true,
            foregroundOcclusion =
              EditorSubPaneForegroundOcclusion(
                height = 330.dp,
                headerRevealHeight = EditorSubPaneHeaderRevealHeight,
              ),
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            SideEffect { exposedKeyboardOcclusion = keyboardOcclusion }
            Box(
              Modifier.testTag(BottomPanelRevealTag)
                .fillMaxWidth()
                .height(EditorSubPaneHeaderRevealHeight)
                .sheetDragHandle()
            )
          }
        }
      }
    }
    waitForIdle()
    assertEquals(398f, checkNotNull(geometry).sheetHeight)

    onNodeWithTag(BottomPanelRevealTag).performTouchInput { click(Offset(x = center.x, y = 12f)) }
    waitForIdle()

    assertEquals(600f, checkNotNull(geometry).sheetHeight)
    assertEquals(0.dp, exposedKeyboardOcclusion)
  }

  @Test
  fun dragHandleTapObserverPreservesChildClick() = runComposeUiTest {
    var childClicks = 0

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 0.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 0.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            Box(Modifier.fillMaxWidth().height(EditorSubPaneHeaderRevealHeight).sheetDragHandle()) {
              Box(
                Modifier.testTag(DragHandleChildButtonTag).size(44.dp).clickable {
                  childClicks += 1
                }
              )
            }
          }
        }
      }
    }
    waitForIdle()

    onNodeWithTag(DragHandleChildButtonTag).performTouchInput { click(center) }
    waitForIdle()

    assertEquals(1, childClicks)
  }

  @Test
  fun draggingBottomPanelRevealResizesFromTemporaryHeight() = runComposeUiTest {
    var geometry: EditorResizableSheetGeometry? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 600.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 0.dp,
            safeBottomInset = 0.dp,
            editorFocused = true,
            foregroundOcclusion =
              EditorSubPaneForegroundOcclusion(
                height = 330.dp,
                headerRevealHeight = EditorSubPaneHeaderRevealHeight,
              ),
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            Box(
              Modifier.testTag(BottomPanelRevealDragHandleTag)
                .fillMaxWidth()
                .height(72.dp)
                .sheetDragHandle()
            )
          }
        }
      }
    }
    waitForIdle()
    assertEquals(398f, checkNotNull(geometry).sheetHeight)

    val handle = onNodeWithTag(BottomPanelRevealDragHandleTag)
    handle.performTouchInput { down(center) }
    try {
      handle.performTouchInput { moveBy(Offset(x = 0f, y = -64f)) }
      waitForIdle()
      val heightAfterDragStarted = checkNotNull(geometry).sheetHeight
      assertTrue(
        heightAfterDragStarted in 398f..490f,
        "drag should start from the temporary 398dp height, but was $heightAfterDragStarted",
      )

      handle.performTouchInput { moveBy(Offset(x = 0f, y = -12f)) }
      waitForIdle()
      val heightAfterNextMove = checkNotNull(geometry).sheetHeight
      assertEquals(12f, heightAfterNextMove - heightAfterDragStarted, absoluteTolerance = 1f)
    } finally {
      handle.performTouchInput { up() }
    }

    waitForIdle()
    assertTrue(checkNotNull(geometry).sheetHeight in 406f..510f)
  }

  @Test
  fun resizableSubPaneParticipatesInToolbarBackdropHazeAboveTheEditorViewport() = runComposeUiTest {
    lateinit var hazeState: HazeState

    setContent {
      val state = remember { HazeState() }
      SideEffect { hazeState = state }
      CompositionLocalProvider(LocalHazeState provides state) {
        ScrollGestureLockScope {
          Box(Modifier.size(width = 400.dp, height = 800.dp)) {
            EditorResizableSheetSurface(
              initialHeight = 360.dp,
              minHeight = 240.dp,
              dismissThreshold = 128.dp,
              maxTopInset = 0.dp,
              trustedImeBottomInset = 0.dp,
              safeBottomInset = 0.dp,
              editorFocused = false,
              minKeyboardVisibleHeight = 0.dp,
              onDismissed = {},
              onGeometryChanged = {},
            ) {
              Box(Modifier.fillMaxSize())
            }
          }
        }
      }
    }
    waitForIdle()

    assertEquals(listOf(1f), hazeState.areas.map { it.zIndex })
  }

  @Test
  fun keyboardOcclusionIsExposedOnlyWhileSheetOwnsFocus() = runComposeUiTest {
    val sheetFocusRequester = FocusRequester()
    var exposedKeyboardOcclusion = (-1).dp

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 320.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            SideEffect { exposedKeyboardOcclusion = keyboardOcclusion }
            Box(
              Modifier.testTag(SheetFocusTag)
                .size(48.dp)
                .focusRequester(sheetFocusRequester)
                .focusable()
            )
          }
        }
      }
    }
    waitForIdle()

    assertEquals(0.dp, exposedKeyboardOcclusion)

    runOnIdle { sheetFocusRequester.requestFocus() }
    waitForIdle()

    assertEquals(320.dp, exposedKeyboardOcclusion)
  }

  @Test
  fun editorOwnedKeyboardCapDoesNotBlockPointerInputAboveRenderedSheet() = runComposeUiTest {
    var backgroundClicks = 0

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          Box(Modifier.testTag(BackgroundTag).fillMaxSize().clickable { backgroundClicks += 1 })
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 180.dp,
            safeBottomInset = 0.dp,
            editorFocused = true,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            Box(Modifier.fillMaxSize())
          }
        }
      }
    }
    waitForIdle()

    onNodeWithTag(BackgroundTag).performTouchInput { click(Offset(x = center.x, y = 500f)) }
    waitForIdle()

    assertEquals(1, backgroundClicks)
  }

  @Test
  fun closingSheetReleasesFocusAndCannotRegainIt() = runComposeUiTest {
    val sheetFocusRequester = FocusRequester()

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 0.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 0.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            Column(Modifier.fillMaxSize()) {
              Box(
                Modifier.testTag(SheetFocusTag)
                  .size(48.dp)
                  .focusRequester(sheetFocusRequester)
                  .focusable()
              )
              Box(Modifier.testTag(DismissButtonTag).size(48.dp).clickable { dismiss() })
            }
            LaunchedEffect(Unit) { sheetFocusRequester.requestFocus() }
          }
        }
      }
    }
    waitForIdle()
    onNodeWithTag(SheetFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    onNodeWithTag(DismissButtonTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    onNodeWithTag(SheetFocusTag).assertIsNotFocused()
    runOnIdle { sheetFocusRequester.requestFocus() }
    mainClock.advanceTimeByFrame()
    waitForIdle()
    onNodeWithTag(SheetFocusTag).assertIsNotFocused()
  }

  @Test
  fun dismissReportsStartBeforeDismissed() = runComposeUiTest {
    val events = mutableListOf<String>()

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 320.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 240.dp,
            onDismissStarted = { events += "start" },
            onDismissed = { events += "dismissed" },
            onGeometryChanged = {},
          ) {
            Box(Modifier.testTag(DismissButtonTag).fillMaxSize().clickable { dismiss() })
          }
        }
      }
    }
    waitForIdle()

    onNodeWithTag(DismissButtonTag).performClick()
    waitForIdle()

    assertEquals(listOf("start", "dismissed"), events)
  }

  @Test
  fun sheetDragHandleDoesNotFlingSheetContentScroll() = runComposeUiTest {
    var scrollState: ScrollState? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 320.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            val state = remember { ScrollState(initial = 1000) }
            scrollState = state

            Column(Modifier.fillMaxSize()) {
              Box(
                Modifier.testTag(SheetDragHandleTag).fillMaxWidth().height(72.dp).sheetDragHandle()
              )
              Column(Modifier.fillMaxSize().verticalScroll(state)) {
                repeat(80) { Box(Modifier.fillMaxWidth().height(48.dp)) }
              }
            }
          }
        }
      }
    }
    waitForIdle()

    val beforeDrag = checkNotNull(scrollState).value

    onNodeWithTag(SheetDragHandleTag).performTouchInput {
      swipeWithVelocity(start = center, end = center + Offset(x = 0f, y = 160f), endVelocity = 900f)
    }
    waitForIdle()

    assertEquals(beforeDrag, checkNotNull(scrollState).value)
  }

  @Test
  fun dragTakesOverImmediatelyDuringKeyboardCapRestoration() = runComposeUiTest {
    var editorFocused by mutableStateOf(true)
    var geometry: EditorResizableSheetGeometry? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 180.dp,
            safeBottomInset = 0.dp,
            editorFocused = editorFocused,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            Box(
              Modifier.testTag(IncrementalDragHandleTag)
                .fillMaxWidth()
                .height(72.dp)
                .sheetDragHandle()
            )
          }
        }
      }
    }
    waitForIdle()
    assertEquals(180f, checkNotNull(geometry).sheetHeight)

    mainClock.autoAdvance = false
    runOnIdle { editorFocused = false }
    repeat(3) { mainClock.advanceTimeByFrame() }
    val restoringHeight = checkNotNull(geometry).sheetHeight
    assertTrue(restoringHeight in 180f..360f && restoringHeight != 180f && restoringHeight != 360f)

    val handle = onNodeWithTag(IncrementalDragHandleTag)
    handle.performTouchInput { down(center) }
    try {
      handle.performTouchInput { moveBy(Offset(x = 0f, y = 64f)) }
      mainClock.advanceTimeByFrame()
      waitForIdle()
      val heightAfterDragStarted = checkNotNull(geometry).sheetHeight
      assertTrue(
        heightAfterDragStarted > restoringHeight,
        "drag should take over from restoration immediately: $restoringHeight -> $heightAfterDragStarted",
      )

      handle.performTouchInput { moveBy(Offset(x = 0f, y = 12f)) }
      mainClock.advanceTimeByFrame()
      waitForIdle()
      val heightAfterNextMove = checkNotNull(geometry).sheetHeight
      assertEquals(12f, heightAfterDragStarted - heightAfterNextMove, absoluteTolerance = 1f)
    } finally {
      handle.performTouchInput { up() }
    }
  }

  @Test
  fun incrementalTouchMovesResizeSheet() = runComposeUiTest {
    var geometry: EditorResizableSheetGeometry? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 0.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 0.dp,
            onDismissed = {},
            onGeometryChanged = { geometry = it },
          ) {
            Box(
              Modifier.testTag(IncrementalDragHandleTag)
                .fillMaxWidth()
                .height(72.dp)
                .sheetDragHandle()
            )
          }
        }
      }
    }
    waitForIdle()

    val beforeDrag = checkNotNull(geometry).sheetHeight

    onNodeWithTag(IncrementalDragHandleTag).performTouchInput {
      val start = center
      down(start)
      repeat(10) { step -> moveTo(start + Offset(x = 0f, y = (step + 1) * 4f)) }
      up()
    }
    waitForIdle()

    val afterDrag = checkNotNull(geometry).sheetHeight
    assertTrue(afterDrag < beforeDrag, "sheet should resize after incremental touch movement")
  }

  @Test
  fun sheetSurfaceBlocksPointerInputBehind() = runComposeUiTest {
    var backgroundClicks = 0

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          Box(Modifier.fillMaxSize().clickable { backgroundClicks += 1 })
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            trustedImeBottomInset = 0.dp,
            safeBottomInset = 0.dp,
            editorFocused = false,
            minKeyboardVisibleHeight = 0.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            Box(Modifier.testTag(SheetSurfaceTag).fillMaxSize())
          }
        }
      }
    }
    waitForIdle()

    onNodeWithTag(SheetSurfaceTag).performTouchInput { click(center) }
    waitForIdle()

    assertEquals(0, backgroundClicks)
  }

  private companion object {
    const val BackgroundTag = "background"
    const val BottomPanelRevealDragHandleTag = "bottom-panel-reveal-drag-handle"
    const val BottomPanelRevealTag = "bottom-panel-reveal"
    const val DismissButtonTag = "dismiss-button"
    const val DragHandleChildButtonTag = "drag-handle-child-button"
    const val IncrementalDragHandleTag = "incremental-drag-handle"
    const val SheetDragHandleTag = "sheet-drag-handle"
    const val SheetSurfaceTag = "sheet-surface"
    const val SheetFocusTag = "sheet-focus"
  }
}
