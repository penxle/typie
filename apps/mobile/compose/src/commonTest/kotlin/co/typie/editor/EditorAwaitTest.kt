package co.typie.editor

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.HistoryTag
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

private val sampleMessage: Message = Message.System(SystemEvent.Initialize)

private fun renderInvalidated(): EditorEvent = EditorEvent.RenderInvalidated

@OptIn(ExperimentalCoroutinesApi::class)
class EditorAwaitTest {
  private val dispatcher = StandardTestDispatcher()

  @BeforeTest
  fun setUp() {
    Dispatchers.setMain(dispatcher)
  }

  @AfterTest
  fun tearDown() {
    Dispatchers.resetMain()
  }

  @Test
  fun initial_state_is_EditorState_Initial() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      assertEquals(EditorState.Initial, editor.state)
      assertEquals(null, editor.cursor)
      assertEquals(null, editor.selection)
      assertEquals(emptyList(), editor.pageSizes)
      assertEquals(null, editor.rootAttrs)
      assertEquals(null, editor.ime)
    }

  @Test
  fun await_enqueues_messages_and_ticks_once() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }

      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun empty_await_block_does_not_tick() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.await { /* nothing */ }

      assertEquals(0, fake.tickCount)
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun await_commits_state_from_tick() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 2f, 3f, 4f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }

      assertEquals(fakeCursor, editor.cursor)
      assertEquals(1L, editor.state.version)
    }

  @Test
  fun await_updates_last_history_tag_from_state_change() =
    runTest(dispatcher) {
      val tag = HistoryTag.PasteHtml(plainText = "hello", start = 3)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.LastHistoryTag))) },
          lastHistoryTagProvider = { tag },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }

      assertEquals(tag, editor.state.lastHistoryTag)
      assertEquals(tag, editor.lastHistoryTag)
    }

  @Test
  fun cursor_exit_with_no_selection_clears_ime_and_delivers_event() =
    runTest(dispatcher) {
      var imeCalls = 0
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.CursorExitedDocumentStart) },
          selectionProvider = { null },
          imeProvider = { _, _ ->
            imeCalls += 1
            error("IME should not be read without an active selection")
          },
        )
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      var cursorExited = 0
      editor.on<EditorEvent.CursorExitedDocumentStart> { _, _ -> cursorExited += 1 }

      editor.await { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(emptyList(), reported)
      assertEquals(null, editor.selection)
      assertEquals(null, editor.ime)
      assertEquals(0, imeCalls)
      assertEquals(1, cursorExited)
    }

  @Test
  fun await_beforeCommit_receives_snapshot_before_state_is_committed() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 2f, 3f, 4f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      var beforeCommitSnapshot: EditorState? = null
      var stateVersionSeenBeforeCommit: Long? = null

      editor.await(
        beforeCommit = { snapshot ->
          beforeCommitSnapshot = snapshot
          stateVersionSeenBeforeCommit = editor.state.version
        }
      ) {
        enqueue(sampleMessage)
      }

      assertEquals(1L, beforeCommitSnapshot?.version)
      assertEquals(fakeCursor, beforeCommitSnapshot?.cursor)
      assertEquals(0L, stateVersionSeenBeforeCommit)
      assertEquals(1L, editor.state.version)
    }

  @Test
  fun snapshot_reads_tracked_ranges_only_when_state_field_changes() =
    runTest(dispatcher) {
      val range =
        TrackedRange(
          id = "comment-1",
          group = "comment",
          anchor = Position(node = "text", offset = 0, affinity = Affinity.Downstream),
          head = Position(node = "text", offset = 4, affinity = Affinity.Downstream),
          metadata = "",
          rects = emptyList(),
          text = "test",
        )
      val rangeEndpoints =
        TrackedRangeEndpoints(
          id = range.id,
          group = range.group,
          anchor = range.anchor,
          head = range.head,
        )
      val events =
        ArrayDeque(
          listOf(
            listOf(EditorEvent.StateChanged(listOf(StateField.TrackedRanges))),
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))),
          )
        )
      val fake =
        FakeFfiEditor(
          onTick = { events.removeFirst() },
          trackedRangesProvider = { listOf(range) },
          trackedRangesContainingPositionProvider = { _, _ -> listOf(rangeEndpoints) },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }

      assertEquals(1, fake.trackedRangesCallCount)
      assertEquals(1, fake.trackedRangesContainingPositionCallCount)
      assertEquals(listOf(range), editor.state.trackedRanges)
      assertEquals(listOf(rangeEndpoints), editor.state.trackedRangesContainingSelectionHead)

      editor.await { enqueue(sampleMessage) }

      assertEquals(1, fake.trackedRangesCallCount)
      assertEquals(1, fake.trackedRangesContainingPositionCallCount)
      assertEquals(listOf(range), editor.state.trackedRanges)
      assertEquals(listOf(rangeEndpoints), editor.state.trackedRangesContainingSelectionHead)
    }

  @Test
  fun snapshot_reads_placeholder_only_when_state_field_changes() =
    runTest(dispatcher) {
      val placeholder =
        PlaceholderMetrics(
          pageIdx = 0,
          rect = Rect(x = 1f, y = 2f, width = 300f, height = 24f),
          fontSize = 1200,
          lineHeight = 160,
          letterSpacing = 0,
          align = Alignment.Left,
        )
      val laterPlaceholder = placeholder.copy(fontSize = 1800, align = Alignment.Right)
      var currentPlaceholder = placeholder
      val events =
        ArrayDeque(
          listOf(
            listOf(EditorEvent.StateChanged(listOf(StateField.Placeholder))),
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))),
          )
        )
      val fake =
        FakeFfiEditor(
          onTick = { events.removeFirst() },
          placeholderProvider = { currentPlaceholder },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }

      assertEquals(1, fake.placeholderCallCount)
      assertEquals(placeholder, editor.placeholder)

      currentPlaceholder = laterPlaceholder
      editor.await { enqueue(sampleMessage) }

      assertEquals(1, fake.placeholderCallCount)
      assertEquals(placeholder, editor.placeholder)
    }

  @Test
  fun await_reports_tick_exception_without_committing() =
    runTest(dispatcher) {
      val boom = RuntimeException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      editor.await { enqueue(sampleMessage) }

      assertEquals(1, reported.size)
      assertTrue(reported.single() is RuntimeException)
      assertEquals(boom.message, reported.single().message)
      assertEquals(EditorState.Initial, editor.state)
    }

  @Test
  fun sync_enqueues_ticks_and_commits_inline() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(5f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.sync { enqueue(sampleMessage) }

      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
      assertEquals(fakeCursor, editor.cursor)
      assertEquals(1L, editor.state.version)
    }

  @Test
  fun insert_template_fragment_calls_inner_ticks_and_commits() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(5f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)
      val payload = byteArrayOf(1, 2, 3)

      val inserted = editor.insertTemplateFragment(payload)

      assertTrue(inserted)
      assertEquals(1, fake.insertedTemplateFragments.size)
      assertContentEquals(payload, fake.insertedTemplateFragments.single())
      assertEquals(1, fake.tickCount)
      assertEquals(fakeCursor, editor.cursor)
      assertEquals(1L, editor.state.version)
    }

  @Test
  fun can_probes_inner_editor_on_demand() =
    runTest(dispatcher) {
      val message: Message = Message.Selection(SelectionOp.Expand(SelectionExpansionUnit.Word))
      val probed = mutableListOf<Message>()
      val fake =
        FakeFfiEditor(
          canProvider = { message ->
            probed += message
            true
          }
        )
      val editor = Editor(fake, this, dispatcher)

      assertTrue(editor.can(message))
      assertEquals(listOf(message), probed)
    }

  @Test
  fun sync_does_not_probe_can_availability() =
    runTest(dispatcher) {
      var canCalls = 0
      val fake =
        FakeFfiEditor(
          canProvider = {
            canCalls += 1
            true
          }
        )
      val editor = Editor(fake, this, dispatcher)

      editor.sync { enqueue(sampleMessage) }

      assertEquals(0, canCalls)
    }

  @Test
  fun sync_reports_tick_exception_without_committing() =
    runTest(dispatcher) {
      val boom = IllegalStateException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      editor.sync { enqueue(sampleMessage) }

      assertEquals(1, reported.size)
      assertTrue(reported.single() is IllegalStateException)
      assertEquals(boom.message, reported.single().message)
      assertEquals(EditorState.Initial, editor.state)
    }

  @Test
  fun sync_wins_over_later_await_commit_via_version_skip() =
    runTest(dispatcher) {
      val cursorA =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { cursorA },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.sync { enqueue(sampleMessage) }
      assertEquals(1L, editor.state.version)
      editor.sync { enqueue(sampleMessage) }
      assertEquals(2L, editor.state.version)
    }

  @Test
  fun listener_receives_event_on_main_dispatcher() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(onTick = { listOf(renderInvalidated()) })
      val editor = Editor(fake, this, dispatcher)

      val received = mutableListOf<EditorEvent>()
      editor.on<EditorEvent.RenderInvalidated> { _, e -> received += e }

      editor.await { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(1, received.size)
      assertTrue(received[0] is EditorEvent.RenderInvalidated)
    }

  @Test
  fun state_changed_events_are_not_delivered_to_listeners() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) })
      val editor = Editor(fake, this, dispatcher)

      var seen = 0
      editor.on<EditorEvent.StateChanged> { _, _ -> seen += 1 }

      editor.await { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(0, seen)
    }

  @Test
  fun unregister_closure_prevents_future_calls() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(onTick = { listOf(renderInvalidated()) })
      val editor = Editor(fake, this, dispatcher)

      var count = 0
      val off = editor.on<EditorEvent.RenderInvalidated> { _, _ -> count += 1 }
      editor.await { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(1, count)

      off()
      editor.await { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(1, count)
    }

  @Test
  fun listener_exception_does_not_block_other_listeners() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(onTick = { listOf(renderInvalidated()) })
      val editor = Editor(fake, this, dispatcher)

      var second = 0
      editor.on<EditorEvent.RenderInvalidated> { _, _ -> error("first boom") }
      editor.on<EditorEvent.RenderInvalidated> { _, _ -> second += 1 }

      editor.await { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(1, second)
    }

  @Test
  fun await_waits_for_onPageSettled_when_RI_is_emitted() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = {
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor)), renderInvalidated())
          },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.attachSurface(page = 0, handle = 0L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()

      assertFalse(returned)
      assertEquals(EditorState.Initial, editor.state)

      editor.onPageSettled(page = 0, version = 1L)
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(returned)
      assertEquals(fakeCursor, editor.cursor)
      job.join()
    }

  @Test
  fun render_skip_settles_page_and_releases_await() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = {
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor)), renderInvalidated())
          },
          cursorProvider = { fakeCursor },
          renderSurfaceProvider = { false },
        )
      val editor = Editor(fake, this, dispatcher)
      editor.attachSurface(page = 0, handle = 0L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(returned)

      // A skipped render presents no new frame, so no bitmap commit (and no
      // onPageSettled from the surface) will ever arrive for this page.
      editor.renderSurface(0)
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(returned)
      assertEquals(fakeCursor, editor.cursor)
      job.join()
    }

  @Test
  fun await_commits_immediately_when_attached_pages_empty() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(0f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(onTick = { listOf(renderInvalidated()) }, cursorProvider = { fakeCursor })
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }

      assertEquals(fakeCursor, editor.cursor)
    }

  @Test
  fun stale_onPageSettled_version_does_not_release_await() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(0f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(onTick = { listOf(renderInvalidated()) }, cursorProvider = { fakeCursor })
      val editor = Editor(fake, this, dispatcher)
      editor.attachSurface(0, 0L, 0.0, 0.0, 1.0)

      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()

      editor.onPageSettled(0, version = 0L) // too old
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(returned)

      editor.onPageSettled(0, version = 1L)
      dispatcher.scheduler.advanceUntilIdle()
      assertTrue(returned)
      job.join()
    }

  @Test
  fun detach_during_wait_releases_settle() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(0f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(onTick = { listOf(renderInvalidated()) }, cursorProvider = { fakeCursor })
      val editor = Editor(fake, this, dispatcher)
      editor.attachSurface(0, 0L, 0.0, 0.0, 1.0)
      editor.attachSurface(1, 0L, 0.0, 0.0, 1.0)

      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()

      editor.onPageSettled(0, version = 1L)
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(returned)

      editor.detachSurface(1)
      dispatcher.scheduler.advanceUntilIdle()
      assertTrue(returned)
      job.join()
    }

  @Test
  fun commit_skips_when_snapshot_version_is_not_newer() =
    runTest(dispatcher) {
      val cursorA =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { cursorA },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }
      assertEquals(cursorA, editor.cursor)
      assertEquals(1L, editor.state.version)

      val cursorB =
        CursorMetrics(pageIdx = 0, caret = Rect(2f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      fake.cursorProvider = { cursorB }
      editor.await { enqueue(sampleMessage) }

      assertEquals(cursorB, editor.cursor)
      assertEquals(2L, editor.state.version)
    }

  @Test
  fun sync_interleaving_during_await_settle_preserves_state_order() =
    runTest(dispatcher) {
      val cursorA =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val cursorB =
        CursorMetrics(pageIdx = 0, caret = Rect(5f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(onTick = { listOf(renderInvalidated()) }, cursorProvider = { cursorA })
      val editor = Editor(fake, this, dispatcher)
      editor.attachSurface(0, 0L, 0.0, 0.0, 1.0)

      val job = launch(dispatcher) { editor.await { enqueue(sampleMessage) } }
      dispatcher.scheduler.advanceUntilIdle()
      // await is waiting for settle (version=1 snapshot pending).

      // sync bumps version to 2 with cursorB — commits immediately.
      fake.cursorProvider = { cursorB }
      editor.sync { enqueue(sampleMessage) }
      assertEquals(cursorB, editor.cursor)
      assertEquals(2L, editor.state.version)

      // Settle arrives for await's version=1 — commit should skip because state is at 2.
      editor.onPageSettled(0, version = 1L)
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(cursorB, editor.cursor)
      assertEquals(2L, editor.state.version)
      job.join()
    }

  @Test
  fun renderSurface_returns_current_version_counter() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) })
      val editor = Editor(fake, this, dispatcher)

      val v0 = editor.renderSurface(0)
      assertEquals(0L, v0)

      editor.sync { enqueue(sampleMessage) }
      val v1 = editor.renderSurface(0)
      assertEquals(1L, v1)
      assertEquals(2, fake.renderCount)
    }

  @Test
  fun await_after_dispose_throws_cancellation() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      editor.dispose()
      assertFailsWith<CancellationException> { editor.await { enqueue(sampleMessage) } }
    }

  @Test
  fun sync_after_dispose_reports_illegal_state() =
    runTest(dispatcher) {
      val reported = mutableListOf<Throwable>()
      val editor =
        Editor(FakeFfiEditor(), this, dispatcher, onError = { _, error -> reported += error })
      editor.dispose()

      editor.sync { enqueue(sampleMessage) }

      assertEquals(listOf("Editor disposed"), reported.map { it.message })
    }

  @Test
  fun reentrant_sync_reports_illegal_state() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      editor.sync {
        enqueue(sampleMessage)
        editor.sync { enqueue(sampleMessage) }
      }

      assertEquals(listOf("nested sync is not supported"), reported.map { it.message })
      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun enqueue_ticks_asynchronously() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.enqueue(sampleMessage)
      assertEquals(0, fake.tickCount) // tick is scheduled, not yet run

      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(1, fake.tickCount)
      assertEquals(listOf(sampleMessage), fake.enqueued)
    }

  @Test
  fun multiple_enqueues_coalesce_into_single_tick() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.enqueue(sampleMessage)
      editor.enqueue(sampleMessage)
      editor.enqueue(sampleMessage)

      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(1, fake.tickCount)
      assertEquals(3, fake.enqueued.size)
    }

  @Test
  fun enqueue_after_dispose_is_silent_noop() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.dispose()

      editor.enqueue(sampleMessage)
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(0, fake.tickCount)
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun await_cancelled_during_settle_still_commits_state() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(onTick = { listOf(renderInvalidated()) }, cursorProvider = { fakeCursor })
      val editor = Editor(fake, this, dispatcher)
      editor.attachSurface(0, 0L, 0.0, 0.0, 1.0)

      val job = launch(dispatcher) { editor.await { enqueue(sampleMessage) } }
      dispatcher.scheduler.advanceUntilIdle()

      job.cancel()
      dispatcher.scheduler.advanceUntilIdle()
      job.join()

      // Even though the caller was cancelled mid-settle, the tick already mutated the native
      // editor, so Kotlin state must follow — otherwise UI goes out of sync with Rust.
      assertEquals(fakeCursor, editor.cursor)
      assertEquals(1L, editor.state.version)
    }

  @Test
  fun await_cancelled_before_resume_keeps_rust_and_kotlin_in_sync() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      val job = launch(dispatcher) { editor.await { enqueue(sampleMessage) } }
      job.cancel()
      dispatcher.scheduler.advanceUntilIdle()
      job.join()

      if (fake.tickCount > 0) {
        assertEquals(1, fake.tickCount)
        assertEquals(1L, editor.state.version)
        assertEquals(fakeCursor, editor.cursor)
      } else {
        assertEquals(EditorState.Initial, editor.state)
      }
    }
}
