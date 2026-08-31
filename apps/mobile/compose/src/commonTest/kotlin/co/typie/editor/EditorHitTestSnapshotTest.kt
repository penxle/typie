package co.typie.editor

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.CalloutVariant
import co.typie.editor.ffi.ClipboardPayload
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.InteractiveRegion
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

class EditorHitTestSnapshotTest {
  private val collapsed =
    Selection(
      anchor = Position(node = "n", offset = 1, affinity = Affinity.Downstream),
      head = Position(node = "n", offset = 1, affinity = Affinity.Downstream),
    )
  private val expanded =
    Selection(
      anchor = Position(node = "n", offset = 0, affinity = Affinity.Downstream),
      head = Position(node = "n", offset = 3, affinity = Affinity.Downstream),
    )

  private fun failingDirectHitTests(fake: FakeFfiEditor) {
    fake.selectionHitProvider = { _, _, _ -> error("direct selectionHitTest FFI must not run") }
    fake.cursorHitProvider = { _, _, _ -> error("direct cursorHitTest FFI must not run") }
    fake.interactiveHitProvider = { _, _, _ -> error("direct interactiveHitTest FFI must not run") }
  }

  @Test
  fun selectionHitTestReadsSnapshotRectsWithInclusiveEdges() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val fake =
        FakeFfiEditor(
          selectionProvider = { expanded },
          selectionHitRectsProvider = {
            listOf(PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 10f, width = 20f, height = 20f)))
          },
        )
      failingDirectHitTests(fake)
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }

      assertTrue(editor.selectionHitTest(page = 0, x = 10f, y = 10f))
      assertTrue(editor.selectionHitTest(page = 0, x = 30f, y = 30f))
      assertFalse(editor.selectionHitTest(page = 0, x = 30.1f, y = 30f))
      assertFalse(editor.selectionHitTest(page = 1, x = 15f, y = 15f))
      assertFalse(editor.cursorHitTest(page = 0, x = 15f, y = 15f))
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun cursorHitTestReadsSnapshotRectsForCollapsedSelection() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val fake =
        FakeFfiEditor(
          selectionProvider = { collapsed },
          cursorHitRectsProvider = {
            listOf(PageRect(pageIdx = 2, rect = Rect(x = 4f, y = 0f, width = 8f, height = 24f)))
          },
        )
      failingDirectHitTests(fake)
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }

      assertTrue(editor.cursorHitTest(page = 2, x = 6f, y = 10f))
      assertFalse(editor.cursorHitTest(page = 0, x = 6f, y = 10f))
      assertFalse(editor.selectionHitTest(page = 2, x = 6f, y = 10f))
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun interactiveHitTestStopsAtFirstContainingEntryRect() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val callout =
        InteractiveRegion(
          pageIdx = 0,
          entryRect = Rect(x = 0f, y = 0f, width = 50f, height = 50f),
          effectiveRect = Rect(x = 40f, y = 40f, width = 5f, height = 5f),
          hit = InteractiveHit.CalloutIcon(id = "callout", nextVariant = CalloutVariant.Info),
        )
      val fold =
        InteractiveRegion(
          pageIdx = 0,
          entryRect = Rect(x = 0f, y = 0f, width = 100f, height = 100f),
          effectiveRect = Rect(x = 0f, y = 0f, width = 100f, height = 100f),
          hit = InteractiveHit.FoldTitle(id = "fold", textRect = null),
        )
      val fake = FakeFfiEditor(interactiveRegionsProvider = { listOf(callout, fold) })
      failingDirectHitTests(fake)
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }

      assertNull(editor.interactiveHitTest(page = 0, x = 10f, y = 10f))
      assertEquals(callout.hit, editor.interactiveHitTest(page = 0, x = 42f, y = 42f))
      assertEquals(fold.hit, editor.interactiveHitTest(page = 0, x = 60f, y = 60f))
      assertNull(editor.interactiveHitTest(page = 1, x = 42f, y = 42f))
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun hitRectsCarryOverUntilGeometryChanges() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var rects =
        listOf(PageRect(pageIdx = 0, rect = Rect(x = 0f, y = 0f, width = 10f, height = 10f)))
      var events: List<EditorEvent> = emptyList()
      val fake =
        FakeFfiEditor(
          onTick = { events },
          selectionProvider = { expanded },
          selectionHitRectsProvider = { rects },
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertTrue(editor.selectionHitTest(page = 0, x = 5f, y = 5f))

      rects =
        listOf(PageRect(pageIdx = 0, rect = Rect(x = 100f, y = 100f, width = 10f, height = 10f)))
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertTrue(
        editor.selectionHitTest(page = 0, x = 5f, y = 5f),
        "rects must carry over when no geometry-affecting event fired",
      )

      events = listOf(EditorEvent.RenderInvalidated)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertFalse(editor.selectionHitTest(page = 0, x = 5f, y = 5f))
      assertTrue(editor.selectionHitTest(page = 0, x = 105f, y = 105f))
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun copySelectionSuspendsThroughEditorMutex() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val payload = ClipboardPayload(html = "<b>x</b>", text = "x")
      val fake = FakeFfiEditor(copySelectionProvider = { payload })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)

      assertEquals(payload, editor.copySelection())

      editor.dispose()
      assertNull(editor.copySelection())
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun failedEditorDoesNotCallCoreForSupplementalReads() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var characterCountCalls = 0
      var copyCalls = 0
      var imeCalls = 0
      val failure = IllegalStateException("render failed")
      val fake =
        FakeFfiEditor(
          characterCountsProvider = {
            characterCountCalls += 1
            FakeFfiEditor.EmptyCharacterCounts
          },
          copySelectionProvider = {
            copyCalls += 1
            null
          },
          imeProvider = { _, _ ->
            imeCalls += 1
            FakeFfiEditor.EmptyIme
          },
          renderSurfaceProvider = { throw failure },
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      // 코어 실패는 onError 로 보고된 뒤 EditorFailureSignal 로 감싸 다시 던져진다(34e5732ee) —
      // 진행 중인 코루틴을 끊기 위한 신호이므로 여기서 받아 둔다.
      val uncaught = mutableListOf<Throwable>()
      val scope =
        CoroutineScope(
          SupervisorJob() + dispatcher + CoroutineExceptionHandler { _, error -> uncaught += error }
        )
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, scope, dispatcher, onError = { _, error -> reported += error })
      editor.activateVisualHost(Any())
      editor.attachSurface(0, 10L, 100.0, 100.0, 1.0, wakeDelivery = {})
      editor.requestSurfacePages(setOf(0))
      advanceUntilIdle()
      editor.setImeSessionActive(true)

      assertNull(editor.characterCounts())
      assertNull(editor.copySelection())
      editor.refreshImeSnapshot()

      assertEquals(listOf<Throwable>(failure), reported)
      assertTrue(uncaught.all { it.unwrapEditorFailureSignal() === failure })
      assertEquals(0, characterCountCalls)
      assertEquals(0, copyCalls)
      assertEquals(0, imeCalls)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }
}
