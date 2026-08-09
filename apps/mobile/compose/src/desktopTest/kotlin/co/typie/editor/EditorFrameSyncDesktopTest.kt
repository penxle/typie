package co.typie.editor

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asSkiaBitmap
import androidx.compose.ui.graphics.compositeOver
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.input.key.Key as ComposeKey
import androidx.compose.ui.platform.InterceptPlatformTextInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorBody
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollIntentResult
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.resolveEditorScrollIntent
import co.typie.editor.scroll.updateNowWithBringIntoView
import co.typie.editor.scroll.updateWithBringIntoView
import co.typie.editor.surface.EditorSurfaceHost
import co.typie.ext.ScrollGestureLockState
import co.typie.screen.editor.editor.layout.EditorScreenLayout
import co.typie.screen.editor.editor.layout.EditorViewportScrollReconcileMode
import co.typie.screen.editor.editor.overlay.resolveSelectionHandleOverlayGeometry
import co.typie.screen.editor.editor.overlay.resolveSelectionHandleOverlayPlacements
import co.typie.screen.editor.editor.state.EditorScreenState
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import java.io.File
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.jetbrains.skia.EncodedImageFormat
import org.jetbrains.skia.Image

@OptIn(ExperimentalComposeUiApi::class, ExperimentalTestApi::class)
class EditorFrameSyncDesktopTest {
  @Test
  fun frameJournalRejectsTextEditWithoutCursorNativeFrame() {
    val sample =
      matchingDesktopFrameSample(repeatKey = RepeatKey.Enter, cursorNativeFrameRevision = null)

    assertTrue(sample.hasPresentationMismatch())
  }

  @Test
  fun frameJournalAllowsNavigationWithoutCursorNativeFrameWhenOverlaysStayAligned() {
    val sample = matchingDesktopFrameSample(cursorNativeFrameRevision = null)

    assertFalse(sample.hasPresentationMismatch())
  }

  @Test
  fun frameJournalRejectsTextEditWithStaleCursorNativeFrame() {
    val sample =
      matchingDesktopFrameSample(
        repeatKey = RepeatKey.Enter,
        revision = 2L,
        cursorNativeFrameRevision = 1L,
      )

    assertTrue(sample.hasPresentationMismatch())
  }

  @Test
  fun frameJournalAllowsFiveConsecutiveFramesWithoutPublication() {
    val revisions = listOf(1L, 1L, 1L, 1L, 1L, 2L, 3L, 4L)
    val samples = RepeatStartPhasesMillis.flatMap { phaseMillis ->
      revisions.mapIndexed { displayFrame, revision ->
        matchingDesktopFrameSample(
          phaseMillis = phaseMillis,
          displayFrame = displayFrame,
          revision = revision,
        )
      }
    }

    assertFrameLiveness(samples)
  }

  @Test
  fun frameJournalRejectsNineConsecutiveFramesWithoutPublication() {
    val revisions = listOf(1L, 1L, 1L, 1L, 1L, 1L, 1L, 1L, 1L, 2L, 3L, 4L)
    val samples = RepeatStartPhasesMillis.flatMap { phaseMillis ->
      revisions.mapIndexed { displayFrame, revision ->
        matchingDesktopFrameSample(
          phaseMillis = phaseMillis,
          displayFrame = displayFrame,
          revision = revision,
        )
      }
    }

    assertFailsWith<AssertionError> { assertFrameLiveness(samples) }
  }

  @Test
  fun physicalArrowKeyRepeatKeepsEveryRenderedPresentationTogether() = runComposeUiTest {
    val fixture = FrameSyncFixture()

    try {
      setFrameSyncContent(fixture)
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }
      fixture.prepareLongDocument(this)
      fixture.editor.focus()
      waitUntil(timeoutMillis = 5_000) { fixture.uiState.focused }
      waitForIdle()

      mainClock.autoAdvance = false
      val journal = DesktopFrameJournal()
      var displayFrame = 0
      for ((phaseIndex, phaseMillis) in RepeatStartPhasesMillis.withIndex()) {
        val phaseStart = journal.samples.size
        repeat(RepeatCyclesPerPhase) {
          repeat(RepeatFramesPerLeg) {
            journal.record(
              driveRepeatDisplayFrame(
                fixture = fixture,
                repeatKey = RepeatKey.Down,
                phaseMillis = phaseMillis,
                displayFrame = displayFrame++,
              )
            )
          }
          repeat(RepeatFramesPerLeg) {
            journal.record(
              driveRepeatDisplayFrame(
                fixture = fixture,
                repeatKey = RepeatKey.Up,
                phaseMillis = phaseMillis,
                displayFrame = displayFrame++,
              )
            )
          }
        }
        journal.record(captureForcedSettleFrame(fixture, phaseMillis, displayFrame++))

        val phaseSamples = journal.samples.subList(phaseStart, journal.samples.size)
        assertTrue(
          phaseSamples.any { it.presentation.scrollY > CoordinateTolerancePx },
          "TEST HARNESS: phase ${phaseMillis}ms never scrolled the long document",
        )
        if (phaseIndex < RepeatStartPhasesMillis.lastIndex) {
          fixture.resetLongDocumentStart(this)
        }
      }

      assertFrameLiveness(journal.samples)
    } finally {
      mainClock.autoAdvance = true
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun physicalEnterAndBackspaceRepeatKeepsEveryRenderedPresentationTogether() = runComposeUiTest {
    val fixture = FrameSyncFixture(continuous = true)

    try {
      setFrameSyncContent(fixture)
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }
      fixture.editor.focus()
      waitUntil(timeoutMillis = 5_000) { fixture.uiState.focused }
      waitForIdle()

      mainClock.autoAdvance = false
      val journal = DesktopFrameJournal()
      var displayFrame = 0
      for (phaseMillis in RepeatStartPhasesMillis) {
        val phaseStart = journal.samples.size
        repeat(EnterBackspaceCyclesPerPhase) {
          repeat(EnterBackspaceRepeatsPerLeg) {
            journal.record(
              driveRepeatDisplayFrame(
                fixture = fixture,
                repeatKey = RepeatKey.Enter,
                phaseMillis = phaseMillis,
                displayFrame = displayFrame++,
              )
            )
          }
          repeat(EnterBackspaceRepeatsPerLeg) {
            journal.record(
              driveRepeatDisplayFrame(
                fixture = fixture,
                repeatKey = RepeatKey.Backspace,
                phaseMillis = phaseMillis,
                displayFrame = displayFrame++,
              )
            )
          }
        }
        journal.record(captureForcedSettleFrame(fixture, phaseMillis, displayFrame++))

        val phaseSamples = journal.samples.subList(phaseStart, journal.samples.size)
        assertTrue(
          phaseSamples
            .map { sample -> sample.presentation.snapshot.pageSizes.sumOf { it.height.toDouble() } }
            .distinct()
            .size > 1,
          "TEST HARNESS: phase ${phaseMillis}ms never changed the continuous scroll extent",
        )
      }

      assertFrameLiveness(journal.samples)
    } finally {
      mainClock.autoAdvance = true
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun firstVisibleContinuousSelectionDrawsHandlesWithoutVirtualizationReset() = runComposeUiTest {
    val fixture =
      FrameSyncFixture(continuous = true, initialDoc = continuousDocumentWithOffscreenTable())

    try {
      setFrameSyncContent(fixture)
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }

      assertTrue(
        fixture.editor.publishedState.pageSizes.size >= 4,
        "TEST HARNESS: continuous document did not span four canvases: " +
          fixture.editor.publishedState.pageSizes,
      )

      val startUpdate =
        assertNotNull(
          fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = 0f, y = 0f)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        )
      waitUntil(timeoutMillis = 10_000) {
        (fixture.editor.publishedRevision ?: -1L) >= startUpdate.revision &&
          fixture.editor.publishedState.cursor?.pageIdx == 0
      }
      waitForIdle()

      val cursor = assertNotNull(fixture.editor.publishedState.cursor)
      val selectionUpdate =
        assertNotNull(
          fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(
              Message.Selection(
                SelectionOp.SelectUnitAt(
                  page = cursor.pageIdx,
                  x = cursor.caret.x + 4f,
                  y = cursor.caret.y + cursor.caret.height / 2f,
                  unit = SelectionPointUnit.Word,
                )
              )
            )
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        )
      waitUntil(timeoutMillis = 10_000) {
        val state = fixture.editor.publishedState
        (fixture.editor.publishedRevision ?: -1L) >= selectionUpdate.revision &&
          !state.selection.isCollapsed() &&
          state.selectionEndpoints != null
      }
      waitForIdle()

      val bundle = assertNotNull(fixture.editor.publishedBundle)
      assertTrue(bundle.frames.containsKey(0))
      assertTrue(bundle.frames.size < bundle.snapshot.pageSizes.size)
      val offscreenTable = assertNotNull(bundle.snapshot.tableOverlays.singleOrNull())
      assertTrue(offscreenTable.pageIdx > 0)
      assertFalse(bundle.frames.containsKey(offscreenTable.pageIdx))
      val editorRect = assertNotNull(fixture.uiState.editorBoundsInContainer.toPxRect(1f))
      val placements =
        assertNotNull(
          resolveSelectionHandleOverlayPlacements(
            state = bundle.snapshot,
            uiState = fixture.uiState,
            editorRectInOverlay = editorRect,
            density = 1f,
          )
        )
      val pixels = onNodeWithTag(RootTag).captureToImage().toPixelMap()
      for (placement in placements) {
        val geometry = resolveSelectionHandleOverlayGeometry(placement, density = 1f)
        val center =
          geometry.touchTargetTopLeft +
            geometry.paintTopLeftInTouchTarget +
            Offset(
              x = geometry.radiusPx,
              y =
                if (placement.type == EditorSelectionHandleType.From) {
                  geometry.radiusPx
                } else {
                  geometry.stemHeightPx + geometry.radiusPx
                },
            )
        assertEquals(
          LightColors.textDefault,
          pixels[center.x.roundToInt(), center.y.roundToInt()],
          "selection handle was not drawn in the first visible frame at $center",
        )
      }
    } finally {
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun commandDocumentNavigationRevealsThePublishedCursor() = runComposeUiTest {
    val fixture = FrameSyncFixture()

    try {
      setFrameSyncContent(fixture)
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }
      fixture.moveToTwoPageEnd(this)
      fixture.editor.updateNow {
        enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = PageMargin, y = PageMargin)))
      }
      fixture.viewportState.scrollToY(0f, isAutoScroll = false)
      fixture.editor.focus()
      waitUntil(timeoutMillis = 5_000) { fixture.uiState.focused }
      waitForIdle()

      val beforeDown = fixture.viewportState.scrollOffset.y
      onNodeWithTag(RootTag).performKeyInput {
        keyDown(ComposeKey.MetaLeft)
        keyDown(ComposeKey.DirectionDown)
        keyUp(ComposeKey.DirectionDown)
        keyUp(ComposeKey.MetaLeft)
      }
      waitForIdle()
      fixture.continuationScheduler.runCurrent()
      waitUntil(timeoutMillis = 10_000) {
        fixture.continuationScheduler.runCurrent()
        fixture.editor.publishedState.cursor?.pageIdx == 1
      }
      waitForIdle()
      fixture.continuationScheduler.runCurrent()
      waitForIdle()
      val downState = fixture.editor.publishedState
      val expectedDown =
        assertNotNull(
          resolveEditorScrollIntent(
            frame = fixture.scrollFrame(downState),
            target = EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.Typewriter,
            currentScroll = beforeDown,
          )
            as? EditorScrollIntentResult.ScrollTo,
          "Cmd+Down did not require a viewport reveal",
        )
      assertTrue(
        abs(fixture.viewportState.scrollOffset.y - expectedDown.y) <= 0.5f,
        "Cmd+Down published cursor=${downState.cursor} at ${downState.version}, " +
          "but viewport=${fixture.viewportState.scrollOffset.y} expected=${expectedDown.y}",
      )

      val beforeUp = fixture.viewportState.scrollOffset.y
      onNodeWithTag(RootTag).performKeyInput {
        keyDown(ComposeKey.MetaLeft)
        keyDown(ComposeKey.DirectionUp)
        keyUp(ComposeKey.DirectionUp)
        keyUp(ComposeKey.MetaLeft)
      }
      waitForIdle()
      fixture.continuationScheduler.runCurrent()
      waitUntil(timeoutMillis = 10_000) {
        fixture.continuationScheduler.runCurrent()
        fixture.editor.publishedState.cursor?.pageIdx == 0
      }
      waitForIdle()
      val upState = fixture.editor.publishedState
      val expectedUp =
        assertNotNull(
          resolveEditorScrollIntent(
            frame = fixture.scrollFrame(upState),
            target = EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.Typewriter,
            currentScroll = beforeUp,
          )
            as? EditorScrollIntentResult.ScrollTo,
          "Cmd+Up did not require a viewport reveal",
        )
      assertTrue(
        abs(fixture.viewportState.scrollOffset.y - expectedUp.y) <= 0.5f,
        "Cmd+Up published cursor=${upState.cursor} at ${upState.version}, " +
          "but viewport=${fixture.viewportState.scrollOffset.y} expected=${expectedUp.y}",
      )
    } finally {
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun initialEntryRestorePublishesWithoutMountedEditorBounds() = runComposeUiTest {
    val fixture = FrameSyncFixture()

    try {
      fixture.editor.updateNow {
        repeat(LongDocumentParagraphCount) { enqueue(Message.Key(KeyEvent(Key.Enter))) }
      }
      assertTrue(fixture.editor.appliedState.pageSizes.size >= 3)
      val saved =
        assertNotNull(
          runBlocking {
            fixture.editor.freezeSelection(assertNotNull(fixture.editor.appliedState.selection))
          }
        )
      fixture.editor.updateNow {
        enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = PageMargin, y = PageMargin)))
      }
      val restore =
        assertNotNull(
          fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(Message.Selection(SelectionOp.SetFrozen(saved)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        )
      assertFalse(fixture.uiState.editorBoundsInContainer.isValid)

      setFrameSyncContent(fixture)
      waitUntil(timeoutMillis = 10_000) {
        val bundle = fixture.editor.publishedBundle ?: return@waitUntil false
        val cursorPage = bundle.snapshot.cursor?.pageIdx ?: return@waitUntil false
        bundle.snapshot.version >= restore.revision && bundle.frames.containsKey(cursorPage)
      }
      waitForIdle()

      val firstRestoredFrame =
        assertNotNull(
          fixture.drawsAfter(0).firstOrNull { it.cursorNativeFrameRevision != null },
          "the App presentation boundary never drew a framed entry presentation",
        )
      assertTrue(firstRestoredFrame.snapshot.version >= restore.revision)
      assertEquals(restore.snapshot.selection, firstRestoredFrame.snapshot.selection)
      assertEquals(
        firstRestoredFrame.snapshot.version,
        firstRestoredFrame.cursorNativeFrameRevision,
      )
      val expectedScroll =
        assertNotNull(
          resolveEditorScrollIntent(
            frame = fixture.scrollFrame(firstRestoredFrame.snapshot),
            target = EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
            currentScroll = 0f,
          )
            as? EditorScrollIntentResult.ScrollTo
        )
      assertTrue(
        abs(firstRestoredFrame.scrollY - expectedScroll.y) <= 0.5f,
        "first restored frame used scroll=${firstRestoredFrame.scrollY}, expected=${expectedScroll.y}",
      )
    } finally {
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun instantRevealUsesTheViewportOfTheAcceptingMeasure() = runComposeUiTest {
    val initialHeight = 300f
    val measuredHeight = 140f
    val fixture = FrameSyncFixture(viewportHeight = initialHeight)
    val rootHeight = mutableFloatStateOf(initialHeight)

    try {
      setFrameSyncContent(fixture, viewportHeight = { rootHeight.floatValue })
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.containsKey(0) == true
      }

      val cursorUpdate = runOnIdle {
        assertNotNull(
          fixture.editor.updateNow { repeat(2) { enqueue(Message.Key(KeyEvent(Key.Enter))) } }
        )
      }
      waitUntil(timeoutMillis = 10_000) {
        (fixture.editor.publishedRevision ?: -1L) >= cursorUpdate.revision
      }
      waitForIdle()

      val target = EditorBringIntoViewTarget.CurrentSelectionHead
      assertEquals(
        EditorScrollIntentResult.NoScroll,
        resolveEditorScrollIntent(
          frame = fixture.scrollFrame(fixture.editor.publishedState),
          target = target,
          policy = EditorBringIntoViewPolicy.CursorGuard,
          currentScroll = 0f,
        ),
        "TEST HARNESS: target must be visible in the stale 300dp viewport",
      )
      assertTrue(
        resolveEditorScrollIntent(
          frame =
            fixture
              .scrollFrame(fixture.editor.publishedState)
              .copy(
                visibleArea =
                  fixture.visibleArea.copy(
                    viewport = fixture.visibleArea.viewport.copy(height = measuredHeight)
                  )
              ),
          target = target,
          policy = EditorBringIntoViewPolicy.CursorGuard,
          currentScroll = 0f,
        )
          is EditorScrollIntentResult.ScrollTo,
        "TEST HARNESS: target must require reveal in the accepting 140dp measure",
      )

      runOnIdle {
        rootHeight.floatValue = measuredHeight
        fixture.bringIntoViewRequests.requestForVersion(
          target = target,
          version = fixture.editor.appliedRevision,
          policy = EditorBringIntoViewPolicy.CursorGuard,
        )
        fixture.editor.requestPublication()
      }

      waitUntil(timeoutMillis = 5_000) { fixture.viewportState.scrollOffset.y > 0f }
    } finally {
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun unfocusedEntryRestorePublishesDestinationPixelsAndScrollInItsFirstFrame() = runComposeUiTest {
    val fixture = FrameSyncFixture()

    try {
      setFrameSyncContent(fixture)
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }
      fixture.moveToTwoPageEnd(this)
      val saved =
        assertNotNull(
          runBlocking {
            fixture.editor.freezeSelection(assertNotNull(fixture.editor.appliedState.selection))
          }
        )
      fixture.resetLongDocumentStart(this)
      assertFalse(fixture.uiState.focused)
      val beforeScroll = fixture.viewportState.scrollOffset.y
      val beforeDraw = fixture.latestDrawSequence()

      val restore =
        assertNotNull(
          fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(Message.Selection(SelectionOp.SetFrozen(saved)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        )
      val expectedScroll =
        assertNotNull(
          resolveEditorScrollIntent(
            frame = fixture.scrollFrame(restore.snapshot),
            target = EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
            currentScroll = beforeScroll,
          )
            as? EditorScrollIntentResult.ScrollTo
        )

      try {
        waitUntil(timeoutMillis = 10_000) {
          (fixture.editor.publishedRevision ?: -1L) >= restore.revision
        }
      } catch (error: Throwable) {
        val editor = fixture.editor
        throw AssertionError(
          "entry restore did not present: restore=${restore.revision} " +
            "applied=${editor.appliedState.version} published=${editor.publishedRevision} " +
            "cursorPage=${editor.appliedState.cursor?.pageIdx} " +
            "required=${editor.surfacePageRequirements} active=${editor.activeSurfacePages} " +
            "publishedFrames=${editor.publishedBundle?.frames?.keys} " +
            "pending=${fixture.bringIntoViewRequests.activateForVersion(editor.appliedState.version)} " +
            "scroll=${fixture.viewportState.scrollOffset.y} " +
            "content=${fixture.viewportState.contentSize} bounds=${fixture.uiState.editorBoundsInContainer}",
          error,
        )
      }
      fixture.forceNextDraw()
      waitForIdle()

      val firstRestoredFrame =
        assertNotNull(
          fixture.drawsAfter(beforeDraw).firstOrNull { it.snapshot.version >= restore.revision },
          "the App presentation boundary never drew a framed restore presentation",
        )
      val cursor = assertNotNull(firstRestoredFrame.snapshot.cursor)
      assertEquals(restore.revision, firstRestoredFrame.snapshot.version)
      assertEquals(restore.revision, firstRestoredFrame.cursorNativeFrameRevision)
      assertTrue(
        abs(firstRestoredFrame.scrollY - expectedScroll.y) <= 0.5f,
        "first restored frame used scroll=${firstRestoredFrame.scrollY}, expected=${expectedScroll.y}",
      )
      assertNotNull(fixture.editor.publishedBundle?.frames?.get(cursor.pageIdx))
      assertFalse(fixture.uiState.focused)
    } finally {
      mainClock.autoAdvance = true
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun continuousEntryRestoreKeepsDestinationSurfaceUntilFirstPublication() = runComposeUiTest {
    val fixture =
      FrameSyncFixture(continuous = true, initialDoc = continuousDocumentWithOffscreenTable())
    var restoreRevision: Long? = null
    var preparationRevisionCount = 0

    try {
      setFrameSyncContent(
        fixture = fixture,
        onRequiredPagesChanged = { requiredPages ->
          val requestedRevision = restoreRevision
          if (
            requestedRevision != null &&
              2 in requiredPages &&
              (fixture.editor.publishedRevision ?: -1L) < requestedRevision
          ) {
            preparationRevisionCount += 1
            fixture.editor.updateNow {
              enqueue(Message.Selection(SelectionOp.SetAt(page = 2, x = 0f, y = 0f)))
            }
          }
        },
      )
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }
      assertTrue(fixture.editor.publishedState.pageSizes.size >= 4)

      val destination =
        assertNotNull(
          fixture.editor.updateNow {
            enqueue(Message.Selection(SelectionOp.SetAt(page = 2, x = 0f, y = 0f)))
          }
        )
      assertEquals(2, destination.snapshot.cursor?.pageIdx)
      val saved =
        assertNotNull(
          runBlocking {
            fixture.editor.freezeSelection(assertNotNull(destination.snapshot.selection))
          }
        )

      val start =
        assertNotNull(
          fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = 0f, y = 0f)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        )
      waitUntil(timeoutMillis = 10_000) {
        (fixture.editor.publishedRevision ?: -1L) >= start.revision
      }
      fixture.viewportState.scrollToY(0f, isAutoScroll = false)
      waitForIdle()

      val beforeDraw = fixture.latestDrawSequence()
      val restore =
        assertNotNull(
          fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(Message.Selection(SelectionOp.SetFrozen(saved)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.CursorGuard,
            )
          }
        )
      restoreRevision = restore.revision
      val expectedScroll =
        assertNotNull(
          resolveEditorScrollIntent(
            frame = fixture.scrollFrame(restore.snapshot),
            target = EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
            currentScroll = 0f,
          )
            as? EditorScrollIntentResult.ScrollTo
        )

      waitUntil(timeoutMillis = 3_000) {
        val bundle = fixture.editor.publishedBundle ?: return@waitUntil false
        bundle.snapshot.version >= restore.revision && bundle.frames.containsKey(2)
      }
      fixture.forceNextDraw()
      waitForIdle()

      assertTrue(preparationRevisionCount > 0, "TEST HARNESS: preparation did not advance")
      val firstRestoredFrame =
        assertNotNull(
          fixture.drawsAfter(beforeDraw).firstOrNull { it.snapshot.version >= restore.revision },
          "the App presentation boundary never drew the continuous entry restore",
        )
      assertTrue(firstRestoredFrame.snapshot.version >= restore.revision)
      assertEquals(restore.snapshot.selection, firstRestoredFrame.snapshot.selection)
      assertEquals(
        firstRestoredFrame.snapshot.version,
        firstRestoredFrame.cursorNativeFrameRevision,
      )
      assertTrue(
        abs(firstRestoredFrame.scrollY - expectedScroll.y) <= 0.5f,
        "first restored frame used scroll=${firstRestoredFrame.scrollY}, expected=${expectedScroll.y}",
      )
    } finally {
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  @Test
  fun enterAcrossPageBoundaryPublishesPixelsAndRevealInOnePresentation() = runComposeUiTest {
    val fixture = FrameSyncFixture()
    var inputRequest: PlatformTextInputMethodRequest? = null

    try {
      setFrameSyncContent(fixture) { inputRequest = it }
      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.isNotEmpty() == true
      }

      fixture.editor.focus()
      waitUntil(timeoutMillis = 5_000) { fixture.uiState.focused && inputRequest != null }
      fixture.moveToLastOnePageState(this)

      mainClock.autoAdvance = false
      val beforeRevision = assertNotNull(fixture.editor.publishedRevision)
      val beforeScroll = fixture.viewportState.scrollOffset.y
      val updateJob =
        fixture.scope.launch {
          fixture.editor.updateWithBringIntoView(fixture.bringIntoViewRequests) {
            enqueue(Message.Key(KeyEvent(Key.Enter)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.Typewriter,
            )
          }
        }
      fixture.continuationScheduler.runCurrent()

      var displayFrames = 0
      while (
        (fixture.editor.publishedRevision ?: beforeRevision) <= beforeRevision &&
          displayFrames < 120
      ) {
        fixture.forceNextDraw()
        mainClock.advanceTimeByFrame()
        waitForIdle()
        displayFrames += 1
      }
      assertTrue(
        (fixture.editor.publishedRevision ?: beforeRevision) > beforeRevision,
        "page-boundary Enter did not present within 120 display frames",
      )
      val bundle = assertNotNull(fixture.editor.publishedBundle)
      val cursor = assertNotNull(bundle.snapshot.cursor)
      val cursorFrameInFirstPresentation = bundle.frames[cursor.pageIdx]

      waitUntil(timeoutMillis = 10_000) {
        fixture.editor.publishedBundle?.frames?.get(cursor.pageIdx) != null
      }
      val materializedBundle = assertNotNull(fixture.editor.publishedBundle)
      val materializedCursorFrame =
        assertNotNull(
          materializedBundle.frames[cursor.pageIdx],
          "STOP: cursor page ${cursor.pageIdx} is not materialized by the current policy",
        )
      assertEquals(
        materializedBundle.snapshot.version,
        materializedCursorFrame.proof.editorRevision,
      )

      val cursorRectInRoot = assertNotNull(fixture.uiState.cursorRectInRoot(cursor))
      val observedInputRequest = assertNotNull(inputRequest)
      assertEquals(cursorRectInRoot, observedInputRequest.focusedRectInRoot())
      val scrollFrame = fixture.scrollFrame(bundle.snapshot)
      val expectedScroll =
        assertNotNull(
          resolveEditorScrollIntent(
            frame = scrollFrame,
            target = EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.Typewriter,
            currentScroll = beforeScroll,
          )
            as? EditorScrollIntentResult.ScrollTo,
          "page-boundary Enter did not require a viewport reveal",
        )
      assertNotNull(
        cursorFrameInFirstPresentation,
        "published snapshot ${bundle.snapshot.version} exposed cursor page ${cursor.pageIdx} " +
          "before that page's native frame was present; the page was materialized afterward " +
          "with frame ${materializedCursorFrame.proof.editorRevision}",
      )
      assertTrue(
        abs(fixture.viewportState.scrollOffset.y - expectedScroll.y) <= 0.5f,
        "published=${bundle.snapshot.version} frame=${materializedCursorFrame.proof.editorRevision} " +
          "page=${cursor.pageIdx} beforeScroll=$beforeScroll " +
          "actualScroll=${fixture.viewportState.scrollOffset.y} expectedScroll=${expectedScroll.y} " +
          "cursorRect=$cursorRectInRoot imeRect=${observedInputRequest.focusedRectInRoot()}",
      )

      fixture.continuationScheduler.runCurrent()
      updateJob.join()
      mainClock.autoAdvance = true

      repeat(3) { cycle ->
        val backspace =
          assertNotNull(
            fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
              enqueue(Message.Key(KeyEvent(Key.Backspace)))
              bringIntoView(
                EditorBringIntoViewTarget.CurrentSelectionHead,
                policy = EditorBringIntoViewPolicy.Typewriter,
              )
            }
          )
        assertEquals(
          1,
          backspace.snapshot.pageSizes.size,
          "cycle $cycle Backspace did not cross 2→1: " +
            "applied=${backspace.snapshot.pageSizes} cursor=${backspace.snapshot.cursor}",
        )
        val shrinking = assertNotNull(fixture.editor.publishedBundle)
        if (shrinking.snapshot.version >= backspace.revision) {
          assertEquals(1, shrinking.snapshot.pageSizes.size)
          assertEquals(0, assertNotNull(shrinking.snapshot.cursor).pageIdx)
          assertNotNull(
            shrinking.frames[0],
            "cycle $cycle published 2→1 geometry without page 0's native frame",
          )
        }
        waitForIdle()
        waitUntil(timeoutMillis = 10_000) {
          (fixture.editor.publishedRevision ?: -1L) >= backspace.revision &&
            fixture.editor.publishedState.pageSizes.size == 1
        }

        if (cycle == 0) {
          val burstEnter =
            assertNotNull(
              fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
                enqueue(Message.Key(KeyEvent(Key.Enter)))
                bringIntoView(
                  EditorBringIntoViewTarget.CurrentSelectionHead,
                  policy = EditorBringIntoViewPolicy.Typewriter,
                )
              }
            )
          assertEquals(2, burstEnter.snapshot.pageSizes.size)
          val burstBackspace =
            assertNotNull(
              fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
                enqueue(Message.Key(KeyEvent(Key.Backspace)))
                bringIntoView(
                  EditorBringIntoViewTarget.CurrentSelectionHead,
                  policy = EditorBringIntoViewPolicy.Typewriter,
                )
              }
            )
          assertEquals(1, burstBackspace.snapshot.pageSizes.size)
          waitForIdle()
          waitUntil(timeoutMillis = 10_000) {
            (fixture.editor.publishedRevision ?: -1L) >= burstBackspace.revision &&
              fixture.editor.publishedState.pageSizes.size == 1
          }
          assertNotNull(fixture.editor.publishedBundle?.frames?.get(0))
        }

        val enter =
          assertNotNull(
            fixture.editor.updateNowWithBringIntoView(fixture.bringIntoViewRequests) {
              enqueue(Message.Key(KeyEvent(Key.Enter)))
              bringIntoView(
                EditorBringIntoViewTarget.CurrentSelectionHead,
                policy = EditorBringIntoViewPolicy.Typewriter,
              )
            }
          )
        val growing = assertNotNull(fixture.editor.publishedBundle)
        val growingCursor = growing.snapshot.cursor
        assertTrue(
          growingCursor?.pageIdx != 1 || growing.frames[1] != null,
          "cycle $cycle published page 1 cursor geometry without its native frame",
        )
        waitForIdle()
        waitUntil(timeoutMillis = 10_000) {
          (fixture.editor.publishedRevision ?: -1L) >= enter.revision &&
            fixture.editor.publishedBundle?.frames?.get(1) != null
        }
      }

      val finalBundle = assertNotNull(fixture.editor.publishedBundle)
      val finalCursor = assertNotNull(finalBundle.snapshot.cursor)
      assertEquals(
        finalBundle.snapshot.version,
        assertNotNull(finalBundle.frames[finalCursor.pageIdx]).proof.editorRevision,
      )
      assertEquals(
        fixture.uiState.cursorRectInRoot(finalCursor),
        observedInputRequest.focusedRectInRoot(),
      )
      assertActualCursorAndLinePixels(fixture, finalCursor)
    } finally {
      mainClock.autoAdvance = true
      fixture.continuationScheduler.runCurrent()
      fixture.close()
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.assertActualCursorAndLinePixels(
    fixture: FrameSyncFixture,
    cursor: co.typie.editor.ffi.CursorMetrics,
  ) {
    val root = onNodeWithTag(RootTag)
    val rootBounds = root.fetchSemanticsNode().boundsInRoot
    val pixels = root.captureToImage().toPixelMap()
    val cursorRect = assertNotNull(fixture.uiState.cursorRectInRoot(cursor))
    val cursorX = (cursorRect.left - rootBounds.left).roundToInt().coerceIn(0, pixels.width - 1)
    val cursorY = (cursorRect.center.y - rootBounds.top).roundToInt().coerceIn(0, pixels.height - 1)
    assertEquals(
      LightColors.textDefault,
      pixels[cursorX, cursorY],
      "actual cursor pixels did not move with the published frame",
    )

    val pageLeft = cursorRect.left - cursor.caret.x
    val lineTop = cursorRect.top - (cursor.caret.y - cursor.line.y)
    val lineX = (pageLeft + 8f - rootBounds.left).roundToInt().coerceIn(0, pixels.width - 1)
    val lineY =
      (lineTop + cursor.line.height / 2f - rootBounds.top)
        .roundToInt()
        .coerceIn(0, pixels.height - 1)
    assertNotEquals(
      LightColors.surfaceDefault,
      pixels[lineX, lineY],
      "actual current-line highlight pixels did not move with the published frame",
    )
  }

  private fun androidx.compose.ui.test.ComposeUiTest.pressPhysicalKeyWithoutPresentationWait(
    key: ComposeKey,
    phaseMillis: Long,
  ) {
    onNodeWithTag(RootTag).performKeyInput {
      if (phaseMillis > 0L) advanceEventTime(phaseMillis)
      keyDown(key)
      keyUp(key)
      val trailingMillis = RepeatIntervalMillis - phaseMillis
      if (trailingMillis > 0L) advanceEventTime(trailingMillis)
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.driveRepeatDisplayFrame(
    fixture: FrameSyncFixture,
    repeatKey: RepeatKey,
    phaseMillis: Long,
    displayFrame: Int,
  ): CapturedDesktopFrame {
    val beforeDraw = fixture.latestDrawSequence()
    pressPhysicalKeyWithoutPresentationWait(repeatKey.key, phaseMillis)
    fixture.continuationScheduler.runCurrent()
    fixture.forceNextDraw()
    mainClock.advanceTimeByFrame()
    waitForIdle()
    return captureDesktopFrameSample(
      fixture = fixture,
      beforeDraw = beforeDraw,
      phaseMillis = phaseMillis,
      displayFrame = displayFrame,
      repeatKey = repeatKey,
    )
  }

  private fun androidx.compose.ui.test.ComposeUiTest.captureForcedSettleFrame(
    fixture: FrameSyncFixture,
    phaseMillis: Long,
    displayFrame: Int,
  ): CapturedDesktopFrame {
    val beforeDraw = fixture.latestDrawSequence()
    fixture.forceNextDraw()
    mainClock.advanceTimeByFrame()
    waitForIdle()
    return captureDesktopFrameSample(
      fixture = fixture,
      beforeDraw = beforeDraw,
      phaseMillis = phaseMillis,
      displayFrame = displayFrame,
      repeatKey = null,
    )
  }

  private fun androidx.compose.ui.test.ComposeUiTest.captureDesktopFrameSample(
    fixture: FrameSyncFixture,
    beforeDraw: Long,
    phaseMillis: Long,
    displayFrame: Int,
    repeatKey: RepeatKey?,
  ): CapturedDesktopFrame {
    val root = onNodeWithTag(RootTag)
    val draws = fixture.drawsAfter(beforeDraw)
    val presentationBeforeCapture =
      draws.lastOrNull()
        ?: assertNotNull(
          fixture.lastDrawnPresentation,
          "TEST HARNESS: no coherent presentation exists before display frame $displayFrame",
        )
    val rootBounds = root.fetchSemanticsNode().boundsInRoot
    val image = root.captureToImage()
    val captureDraws = fixture.drawsAfter(presentationBeforeCapture.sequence)
    for (draw in draws + captureDraws) {
      if (
        draw.snapshot.cursor != null &&
          (repeatKey == RepeatKey.Enter || repeatKey == RepeatKey.Backspace)
      ) {
        assertEquals(
          draw.snapshot.version,
          draw.cursorNativeFrameRevision,
          "capture-triggered draw ${draw.sequence} mixed snapshot and native frame revisions",
        )
      }
    }
    // captureToImage may itself service an already-invalidated draw. Its pixels correspond
    // to the last such draw, so compare them with that presentation rather than rejecting a
    // coherent capture merely because the test API flushed pending work.
    val presentation = captureDraws.lastOrNull() ?: presentationBeforeCapture

    val cursor = assertNotNull(presentation.snapshot.cursor)
    val bodyTop =
      resolveEditorBodyGeometry(
          visibleArea = fixture.visibleArea,
          layoutSpec = fixture.layoutSpec,
          pageSizes = presentation.snapshot.pageSizes,
        )
        .topSpacerHeight
    val pageTop =
      assertNotNull(
        fixture.layoutSpec.resolvePageContentTop(
          page = cursor.pageIdx,
          pageSizes = presentation.snapshot.pageSizes,
          displayZoom = 1f,
          density = 1f,
        )
      )
    val cursorDocumentY = bodyTop + pageTop + cursor.caret.y
    val highlightDocumentY = bodyTop + pageTop + cursor.line.y
    val expectedCursorY = rootBounds.top + cursorDocumentY - presentation.scrollY
    val expectedHighlightY = rootBounds.top + highlightDocumentY - presentation.scrollY
    val pixels = image.toPixelMap()
    val pageLeft = (pixels.width - presentation.snapshot.pageSizes[cursor.pageIdx].width) / 2f
    val cursorX =
      (pageLeft + cursor.caret.x + cursor.caret.width / 2f)
        .roundToInt()
        .coerceIn(0, pixels.width - 1)
    val highlightX = (pageLeft + HighlightSampleInset).roundToInt().coerceIn(0, pixels.width - 1)
    val actualCursorY =
      ((cursorX - CursorScanRadius).coerceAtLeast(0)..(cursorX + CursorScanRadius).coerceAtMost(
            pixels.width - 1
          ))
        .mapNotNull { x ->
          findMatchingVerticalBandTop(
            imageHeight = pixels.height,
            expectedTop = expectedCursorY - rootBounds.top,
            expectedHeight = null,
          ) { y ->
            colorsApproximatelyEqual(pixels[x, y], LightColors.textDefault)
          }
        }
        .minByOrNull { top -> abs(top - (expectedCursorY - rootBounds.top)) }
        ?.plus(rootBounds.top)
    val highlightColor =
      LightColors.surfaceInverse.copy(alpha = 0.04f).compositeOver(LightColors.surfaceDefault)
    val actualHighlightY =
      findMatchingVerticalBandTop(
          imageHeight = pixels.height,
          expectedTop = expectedHighlightY - rootBounds.top,
          expectedHeight = cursor.line.height,
        ) { y ->
          colorsApproximatelyEqual(pixels[highlightX, y], highlightColor)
        }
        ?.plus(rootBounds.top)

    return CapturedDesktopFrame(
      sample =
        DesktopFrameSample(
          phaseMillis = phaseMillis,
          displayFrame = displayFrame,
          repeatKey = repeatKey,
          presentation = presentation,
          drawPassCount = draws.size + captureDraws.size,
          cursorDocumentY = cursorDocumentY,
          highlightDocumentY = highlightDocumentY,
          expectedCursorY = expectedCursorY,
          actualCursorY = actualCursorY,
          expectedHighlightY = expectedHighlightY,
          actualHighlightY = actualHighlightY,
        ),
      image = image,
    )
  }

  private fun findMatchingVerticalBandTop(
    imageHeight: Int,
    expectedTop: Float,
    expectedHeight: Float?,
    matches: (Int) -> Boolean,
  ): Float? {
    val runs = mutableListOf<IntRange>()
    var runStart: Int? = null
    for (y in 0 until imageHeight) {
      if (matches(y)) {
        if (runStart == null) runStart = y
      } else if (runStart != null) {
        runs += runStart..(y - 1)
        runStart = null
      }
    }
    if (runStart != null) runs += runStart..(imageHeight - 1)

    return runs
      .filter { run ->
        expectedHeight == null ||
          abs(run.count() - expectedHeight.roundToInt().coerceAtLeast(1)) <=
            PixelBandHeightTolerance
      }
      .minByOrNull { run -> abs(run.first - expectedTop) }
      ?.first
      ?.toFloat()
  }

  private fun colorsApproximatelyEqual(actual: Color, expected: Color): Boolean =
    abs(actual.red - expected.red) <= ColorChannelTolerance &&
      abs(actual.green - expected.green) <= ColorChannelTolerance &&
      abs(actual.blue - expected.blue) <= ColorChannelTolerance &&
      abs(actual.alpha - expected.alpha) <= ColorChannelTolerance

  private fun matchingDesktopFrameSample(
    phaseMillis: Long = 0L,
    displayFrame: Int = 0,
    repeatKey: RepeatKey = RepeatKey.Down,
    revision: Long = 1L,
    cursorNativeFrameRevision: Long? = revision,
  ): DesktopFrameSample =
    DesktopFrameSample(
      phaseMillis = phaseMillis,
      displayFrame = displayFrame,
      repeatKey = repeatKey,
      presentation =
        DrawnPresentation(
          sequence = displayFrame.toLong(),
          snapshot = EditorState.Initial.copy(version = revision),
          scrollY = 0f,
          cursorNativeFrameRevision = cursorNativeFrameRevision,
          editorBoundsY = 0f,
          cursorPageOffsetY = 0f,
          testDrawTick = displayFrame,
        ),
      drawPassCount = 1,
      cursorDocumentY = 0f,
      highlightDocumentY = 0f,
      expectedCursorY = 0f,
      actualCursorY = 0f,
      expectedHighlightY = 0f,
      actualHighlightY = 0f,
    )

  private inner class DesktopFrameJournal {
    val samples = mutableListOf<DesktopFrameSample>()
    private var previous: CapturedDesktopFrame? = null

    fun record(frame: CapturedDesktopFrame) {
      val sample = frame.sample
      if (sample.hasPresentationMismatch()) {
        val artifactDirectory = writeDesktopFailureArtifacts(listOfNotNull(previous, frame))
        error(
          "Key-repeat presentation mismatch; artifacts=${artifactDirectory.absolutePath}\n" +
            formatDesktopFrameSample(sample, previous?.sample)
        )
      }
      samples += sample
      previous = frame
    }
  }

  private fun DesktopFrameSample.hasPresentationMismatch(): Boolean {
    val nativeFrameMismatch =
      when (repeatKey) {
        RepeatKey.Enter,
        RepeatKey.Backspace ->
          presentation.cursorNativeFrameRevision != presentation.snapshot.version
        RepeatKey.Down,
        RepeatKey.Up,
        null -> false
      }
    return nativeFrameMismatch ||
      actualCursorY == null ||
      abs(actualCursorY - expectedCursorY) > CoordinateTolerancePx ||
      actualHighlightY == null ||
      abs(actualHighlightY - expectedHighlightY) > CoordinateTolerancePx
  }

  private fun assertFrameLiveness(samples: List<DesktopFrameSample>) {
    for (phaseMillis in RepeatStartPhasesMillis) {
      val active = samples.filter { it.phaseMillis == phaseMillis && it.repeatKey != null }
      var previousRevision: Long? = null
      var sameRevisionFrames = 0
      var maximumSameRevisionFrames = 0
      for (sample in active) {
        val revision = sample.presentation.snapshot.version
        sameRevisionFrames = if (revision == previousRevision) sameRevisionFrames + 1 else 1
        previousRevision = revision
        maximumSameRevisionFrames = maxOf(maximumSameRevisionFrames, sameRevisionFrames)
      }
      val maximumPublicationStallMillis =
        (maximumSameRevisionFrames - 1).coerceAtLeast(0) * RepeatIntervalMillis
      assertTrue(
        maximumPublicationStallMillis <= MaximumPublicationStallMillis,
        "Key-repeat publication stalled at phase ${phaseMillis}ms: " +
          "maximumPublicationStallMillis=$maximumPublicationStallMillis " +
          "revisions=${active.map { it.presentation.snapshot.version }}",
      )
    }
  }

  private fun writeDesktopFailureArtifacts(frames: List<CapturedDesktopFrame>): File {
    val directory =
      File(
          System.getProperty("java.io.tmpdir"),
          "typie-editor-frame-sync/${System.currentTimeMillis()}-${System.nanoTime()}",
        )
        .apply { mkdirs() }
    File(directory, "journal.txt")
      .writeText(
        frames
          .mapIndexed { index, frame ->
            formatDesktopFrameSample(frame.sample, frames.getOrNull(index - 1)?.sample)
          }
          .joinToString(separator = "\n\n")
      )
    for ((sample, image) in frames) {
      val bytes =
        Image.makeFromBitmap(image.asSkiaBitmap()).encodeToData(EncodedImageFormat.PNG)?.bytes
          ?: error("Could not encode frame PNG")
      File(directory, "frame-${sample.displayFrame}.png").writeBytes(bytes)
    }
    return directory
  }

  private fun formatDesktopFrameSample(
    sample: DesktopFrameSample,
    previous: DesktopFrameSample?,
  ): String {
    val cursor = sample.presentation.snapshot.cursor
    val cursorDocumentDelta = previous?.let { sample.cursorDocumentY - it.cursorDocumentY }
    val highlightDocumentDelta = previous?.let { sample.highlightDocumentY - it.highlightDocumentY }
    val scrollDelta = previous?.let { sample.presentation.scrollY - it.presentation.scrollY }
    val cursorViewportDelta =
      previous?.actualCursorY?.let { previousY ->
        sample.actualCursorY?.let { currentY -> currentY - previousY }
      }
    val highlightViewportDelta =
      previous?.actualHighlightY?.let { previousY ->
        sample.actualHighlightY?.let { currentY -> currentY - previousY }
      }
    return buildString {
      appendLine(
        "phase=${sample.phaseMillis}ms displayFrame=${sample.displayFrame} " +
          "key=${sample.repeatKey} draw=${sample.presentation.sequence} " +
          "drawPasses=${sample.drawPassCount} revision=${sample.presentation.snapshot.version}"
      )
      appendLine(
        "scrollY=${sample.presentation.scrollY} cursorPage=${cursor?.pageIdx} " +
          "cursorNativeFrameRevision=${sample.presentation.cursorNativeFrameRevision ?: "missing"}"
      )
      appendLine(
        "editorBoundsY=${sample.presentation.editorBoundsY} " +
          "cursorPageOffsetY=${sample.presentation.cursorPageOffsetY}"
      )
      appendLine(
        "cursor expected=${sample.expectedCursorY} actual=${sample.actualCursorY} " +
          "highlight expected=${sample.expectedHighlightY} actual=${sample.actualHighlightY}"
      )
      append(
        "delta cursorDocument=$cursorDocumentDelta highlightDocument=$highlightDocumentDelta " +
          "scroll=$scrollDelta cursorViewport=$cursorViewportDelta " +
          "highlightViewport=$highlightViewportDelta"
      )
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setFrameSyncContent(
    fixture: FrameSyncFixture,
    viewportHeight: () -> Float = { fixture.visibleArea.viewport.height },
    onRequiredPagesChanged: (Set<Int>) -> Unit = {},
    onInputRequest: (PlatformTextInputMethodRequest) -> Unit = {},
  ) {
    setContent {
      InterceptPlatformTextInput(
        interceptor = { request, nextHandler ->
          onInputRequest(request)
          nextHandler.startInputMethod(request)
        }
      ) {
        val interactionScope = remember { EditorInteractionScope(fixture.scope) }
        val scrollGestureLockState = remember { ScrollGestureLockState() }
        val zoomController = remember { EditorZoomController() }
        val publishedBundle = fixture.editor.publishedBundle
        val publishedState = publishedBundle?.snapshot ?: EditorState.Initial
        val geometry =
          resolveEditorBodyGeometry(
            visibleArea = fixture.visibleArea,
            layoutSpec = fixture.layoutSpec,
            pageSizes = publishedState.pageSizes,
          )
        val forceDrawTick = fixture.forceDrawTick.intValue
        LaunchedEffect(fixture.editor, onRequiredPagesChanged) {
          snapshotFlow { fixture.editor.surfacePageRequirements }
            .collect { onRequiredPagesChanged(it) }
        }
        val scrollFrame = fixture.scrollFrame(publishedState)
        val viewportScrollableState = rememberScrollable2DState { delta ->
          val consumed = fixture.viewportState.consumePan(Offset(x = -delta.x, y = -delta.y))
          Offset(x = -consumed.x, y = -consumed.y)
        }

        SideEffect {
          interactionScope.update(
            editor = fixture.editor,
            bringIntoViewRequests = fixture.bringIntoViewRequests,
            uiState = fixture.uiState,
            visibleArea = fixture.visibleArea,
            viewportState = fixture.viewportState,
            density = 1f,
            scrollGestureLockState = scrollGestureLockState,
            viewportZoomConfig = null,
            layoutSpec = fixture.layoutSpec,
            onSelectionHaptic = {},
            onRequestSoftwareKeyboard = {},
          )
        }

        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalEditorRuntime provides fixture.runtime,
          LocalEditorUiState provides fixture.uiState,
          LocalEditorZoomController provides zoomController,
          LocalEditorBringIntoViewRequests provides fixture.bringIntoViewRequests,
          LocalEditorInteractionScope provides interactionScope,
        ) {
          EditorSurfaceHost(
            editor = fixture.editor,
            scaleFactor = 1.0,
            onDeactivate = fixture.bringIntoViewRequests::cancel,
            onFailure = { throw it },
          )
          EditorScreenLayout(
            state = remember { EditorScreenState(fixture.viewportState) },
            editor = fixture.editor,
            scrollFrame = scrollFrame,
            visibleArea = fixture.visibleArea,
            viewportScrollableState = viewportScrollableState,
            viewportContentWidth = geometry.pageColumnWidth,
            viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
            onMeasuredViewportSizeChange = {},
            header = {},
            body = { presentedBundle ->
              val presentedState = presentedBundle?.snapshot ?: EditorState.Initial
              EditorBody(
                load = fixture.load,
                publishedBundle = presentedBundle,
                visibleArea = fixture.visibleArea,
                layoutSpec = fixture.layoutSpec,
                autoScrollPolicy = fixture.autoScrollPolicy,
                editorInputEnabled = true,
                suppressSoftwareKeyboard = true,
                modifier =
                  Modifier.drawWithContent {
                    drawContent()
                    val cursorPage = presentedState.cursor?.pageIdx
                    fixture.recordDraw(
                      snapshot = presentedState,
                      scrollY = fixture.viewportState.scrollOffset.y,
                      cursorNativeFrameRevision =
                        cursorPage?.let { presentedBundle?.frames?.get(it)?.proof?.editorRevision },
                      testDrawTick = forceDrawTick,
                    )
                  },
              )
            },
            toolbar = {},
            modifier =
              Modifier.size(ViewportWidth.dp, viewportHeight().dp)
                .background(LightColors.surfaceDefault)
                .testTag(RootTag),
          )
        }
      }
    }
  }
}
