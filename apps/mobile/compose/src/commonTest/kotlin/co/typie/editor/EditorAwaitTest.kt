package co.typie.editor

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.HistoryTag
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.ProseRangeInstallOutcome
import co.typie.editor.ffi.ProseTrackedRangeRegistration
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import kotlin.coroutines.CoroutineContext
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.withContext

private val sampleMessage: Message = Message.System(SystemEvent.Initialize)

private fun renderInvalidated(): EditorEvent = EditorEvent.RenderInvalidated

private fun proseRange(
  id: String,
  group: String,
  start: Int,
  end: Int,
): ProseTrackedRangeRegistration =
  ProseTrackedRangeRegistration(id = id, group = group, start = start, end = end)

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
  fun prose_range_install_returns_the_single_result() =
    runTest(dispatcher) {
      lateinit var fake: FakeFfiEditor
      fake =
        FakeFfiEditor(
          onTick = {
            listOf(EditorEvent.ProseRangeInstallResult(outcome = ProseRangeInstallOutcome.Applied))
          }
        )
      val editor = Editor(fake, this, dispatcher)

      val result =
        editor.replaceTrackedRangeGroupsFromProse(
          expectedText = "hello",
          groups = listOf("spellcheck"),
          ranges = listOf(proseRange("result", "spellcheck", 0, 5)),
          isCurrent = { true },
        )

      assertEquals(ProseRangeInstallOutcome.Applied, result)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun prose_range_install_rejects_before_enqueue_when_admission_rejects() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val result =
        editor.replaceTrackedRangeGroupsFromProse(
          expectedText = "hello",
          groups = listOf("spellcheck"),
          ranges = listOf(proseRange("result", "spellcheck", 0, 5)),
          isCurrent = { false },
        )

      assertEquals(null, result)
      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
      assertEquals(0L, editor.state.version)
    }

  @Test
  fun prose_range_install_becomes_superseded_when_admission_changes_after_tick() =
    runTest(dispatcher) {
      var current = true
      lateinit var fake: FakeFfiEditor
      fake =
        FakeFfiEditor(
          onTick = {
            current = false
            listOf(EditorEvent.ProseRangeInstallResult(outcome = ProseRangeInstallOutcome.Applied))
          }
        )
      val editor = Editor(fake, this, dispatcher)

      val result =
        editor.replaceTrackedRangeGroupsFromProse(
          expectedText = "hello",
          groups = listOf("spellcheck"),
          ranges = listOf(proseRange("result", "spellcheck", 0, 5)),
          isCurrent = { current },
        )

      assertEquals(null, result)
      assertEquals(1L, editor.state.version)
    }

  @Test
  fun prose_range_install_missing_or_duplicate_result_is_a_contract_failure() =
    runTest(dispatcher) {
      val missing = Editor(FakeFfiEditor(), this, dispatcher)
      assertFailsWith<IllegalStateException> {
        missing.replaceTrackedRangeGroupsFromProse(
          expectedText = "hello",
          groups = listOf("spellcheck"),
          ranges = emptyList(),
          isCurrent = { true },
        )
      }

      lateinit var fake: FakeFfiEditor
      fake =
        FakeFfiEditor(
          onTick = {
            List(2) {
              EditorEvent.ProseRangeInstallResult(outcome = ProseRangeInstallOutcome.Applied)
            }
          }
        )
      val duplicate = Editor(fake, this, dispatcher)
      assertFailsWith<IllegalStateException> {
        duplicate.replaceTrackedRangeGroupsFromProse(
          expectedText = "hello",
          groups = listOf("spellcheck"),
          ranges = emptyList(),
          isCurrent = { true },
        )
      }
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
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.CursorExitedDocumentStart) },
          selectionProvider = { null },
          imeProvider = { _, _ -> null },
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
  fun document_revision_advances_only_when_doc_field_changes() =
    runTest(dispatcher) {
      val events =
        ArrayDeque(
          listOf(
            listOf(EditorEvent.StateChanged(listOf(StateField.Doc))),
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))),
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor, StateField.Doc))),
          )
        )
      val fake = FakeFfiEditor(onTick = { events.removeFirst() })
      val editor = Editor(fake, this, dispatcher)

      editor.await { enqueue(sampleMessage) }
      assertEquals(1L, editor.state.version)
      assertEquals(1L, editor.state.documentRevision)

      // cursor-only tick: version advances, documentRevision stays
      editor.await { enqueue(sampleMessage) }
      assertEquals(2L, editor.state.version)
      assertEquals(1L, editor.state.documentRevision)

      editor.await { enqueue(sampleMessage) }
      assertEquals(3L, editor.state.version)
      assertEquals(2L, editor.state.documentRevision)
    }

  @Test
  fun await_reports_and_propagates_tick_exception_without_committing() =
    runTest(dispatcher) {
      val boom = RuntimeException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      val thrown = assertFailsWith<RuntimeException> { editor.await { enqueue(sampleMessage) } }

      assertEquals(boom.message, thrown.message)
      assertEquals(1, reported.size)
      assertTrue(reported.single() is RuntimeException)
      assertEquals(boom.message, reported.single().message)
      assertEquals(EditorState.Initial, editor.state)
    }

  @Test
  fun await_is_rejected_after_local_transactions_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.quiesceLocalEdits()

      assertFailsWith<CancellationException> { editor.await { enqueue(sampleMessage) } }

      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
    }

  @Test
  fun track_local_edit_registers_before_its_coroutine_starts() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      val gate = CompletableDeferred<Unit>()

      editor.trackLocalEdit { context -> launch(context) { gate.await() } }
      val quiescence = editor.quiesceLocalEdits()
      val barrier = async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }

      assertFalse(barrier.isCompleted)

      dispatcher.scheduler.runCurrent()
      assertFalse(barrier.isCompleted)

      gate.complete(Unit)
      dispatcher.scheduler.runCurrent()
      assertTrue(barrier.await().isSuccess)
    }

  @Test
  fun track_local_edit_is_rejected_after_quiesce() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      var started = false
      editor.quiesceLocalEdits()

      editor.trackLocalEdit { context -> launch(context) { started = true } }
      dispatcher.scheduler.advanceUntilIdle()

      assertFalse(started)
    }

  @Test
  fun accepted_tracked_local_edit_can_commit_after_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.trackLocalEdit { context ->
        launch(context) { editor.await { enqueue(sampleMessage) } }
      }
      val quiescence = editor.quiesceLocalEdits()
      val barrier = async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }

      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(barrier.await().isSuccess)
      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun accepted_local_edit_can_commit_from_another_coroutine_after_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val handedOff = CompletableDeferred<CoroutineContext>()
      val completion = CompletableDeferred<Unit>()

      editor.trackLocalEdit { context ->
        handedOff.complete(context)
        completion
      }
      val context = handedOff.await()
      val quiescence = editor.quiesceLocalEdits()
      val barrier = async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }

      assertFalse(barrier.isCompleted)

      withContext(context) { editor.await { enqueue(sampleMessage) } }
      completion.complete(Unit)

      assertTrue(barrier.await().isSuccess)
      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
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
  fun sync_is_rejected_after_local_transactions_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.quiesceLocalEdits()

      editor.sync { enqueue(sampleMessage) }

      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
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
  fun insert_template_fragment_is_rejected_after_local_edits_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val quiescence = editor.quiesceLocalEdits()

      try {
        assertFailsWith<CancellationException> {
          editor.insertTemplateFragment(byteArrayOf(1, 2, 3))
        }
        assertTrue(fake.insertedTemplateFragments.isEmpty())
        assertEquals(0, fake.tickCount)
      } finally {
        quiescence.resume()
      }
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
  fun requested_render_skip_settles_page_and_releases_await() =
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
      val surface =
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
      val presentedVersions = mutableListOf<Long>()
      surface.requestRender { presentedVersions += it }
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(returned)
      assertEquals(fakeCursor, editor.cursor)
      assertEquals(emptyList(), presentedVersions)
      job.join()
    }

  @Test
  fun surface_render_requests_are_deferred_and_coalesced() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val firstCallbackVersions = mutableListOf<Long>()
      val latestCallbackVersions = mutableListOf<Long>()
      val surface =
        editor.attachSurface(page = 0, handle = 1L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      surface.requestRender { firstCallbackVersions += it }
      surface.requestRender { latestCallbackVersions += it }

      assertEquals(0, fake.renderCount)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(1, fake.renderCount)
      assertEquals(emptyList(), firstCallbackVersions)
      assertEquals(listOf(0L), latestCallbackVersions)
    }

  @Test
  fun surface_resize_request_replaces_pending_render_and_renders_latest_size() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val presentedVersions = mutableListOf<Long>()
      val surface =
        editor.attachSurface(page = 0, handle = 1L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      surface.requestRender { presentedVersions += it }
      surface.requestResize(SurfaceConfiguration(width = 10.0, height = 20.0, scaleFactor = 2.0)) {
        presentedVersions += it
      }
      surface.requestResize(SurfaceConfiguration(width = 12.0, height = 24.0, scaleFactor = 3.0)) {
        presentedVersions += it
      }

      assertEquals(0, fake.renderCount)
      assertEquals(emptyList(), fake.resizeCalls)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(
        listOf(
          FakeFfiEditor.SurfaceResizeCall(page = 0, width = 12.0, height = 24.0, scaleFactor = 3.0)
        ),
        fake.resizeCalls,
      )
      assertEquals(1, fake.renderCount)
      assertEquals(listOf(0L), presentedVersions)
    }

  @Test
  fun render_settles_without_rendering_when_engine_page_size_differs_from_surface() =
    runTest(dispatcher) {
      var pageSize = Size(width = 1f, height = 2f)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(pageSize) },
        )
      val editor = Editor(fake, this, dispatcher)
      val surface =
        editor.attachSurface(page = 0, handle = 1L, width = 1.0, height = 2.0, scaleFactor = 1.0)
      dispatcher.scheduler.advanceUntilIdle()

      pageSize = Size(width = 3f, height = 4f)
      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(returned)

      surface.requestRender { version -> editor.onPageSettled(page = 0, version = version) }
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(returned)
      assertEquals(0, fake.renderCount)
      job.join()
    }

  @Test
  fun stale_surface_resize_settles_without_rendering() =
    runTest(dispatcher) {
      var pageSize = Size(width = 1f, height = 2f)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.RenderInvalidated) },
          pageSizesProvider = { listOf(pageSize) },
        )
      val editor = Editor(fake, this, dispatcher)
      val surface =
        editor.attachSurface(page = 0, handle = 1L, width = 1.0, height = 2.0, scaleFactor = 1.0)
      dispatcher.scheduler.advanceUntilIdle()

      pageSize = Size(width = 3f, height = 4f)
      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(returned)

      surface.requestResize(SurfaceConfiguration(width = 1.0, height = 2.0, scaleFactor = 1.0)) {
        version ->
        editor.onPageSettled(page = 0, version = version)
      }
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(returned)
      assertEquals(
        listOf(
          FakeFfiEditor.SurfaceResizeCall(page = 0, width = 1.0, height = 2.0, scaleFactor = 1.0)
        ),
        fake.resizeCalls,
      )
      assertEquals(0, fake.renderCount)
      job.join()
    }

  @Test
  fun detach_surface_drops_pending_surface_render_request() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val presentedVersions = mutableListOf<Long>()

      val surface =
        editor.attachSurface(page = 0, handle = 1L, width = 0.0, height = 0.0, scaleFactor = 1.0)
      surface.requestRender { presentedVersions += it }
      surface.detach()

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(0, fake.renderCount)
      assertEquals(emptyList(), presentedVersions)
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
      val pageOne = editor.attachSurface(1, 1L, 0.0, 0.0, 1.0)

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

      pageOne.detach()
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
  fun requested_surface_render_reports_current_version_counter_when_frame_is_presented() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) })
      val editor = Editor(fake, this, dispatcher)
      val presentedVersions = mutableListOf<Long>()
      val surface =
        editor.attachSurface(page = 0, handle = 1L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      surface.requestRender { presentedVersions += it }
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(listOf(0L), presentedVersions)

      editor.sync { enqueue(sampleMessage) }
      surface.requestRender { presentedVersions += it }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf(0L, 1L), presentedVersions)
      assertEquals(2, fake.renderCount)
    }

  @Test
  fun stale_surface_session_does_not_present_after_reattach() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val staleVersions = mutableListOf<Long>()
      val currentVersions = mutableListOf<Long>()

      val stale =
        editor.attachSurface(page = 0, handle = 1L, width = 0.0, height = 0.0, scaleFactor = 1.0)
      stale.requestRender { staleVersions += it }
      stale.detach()
      val current =
        editor.attachSurface(page = 0, handle = 2L, width = 0.0, height = 0.0, scaleFactor = 1.0)
      current.requestRender { currentVersions += it }

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(emptyList(), staleVersions)
      assertEquals(listOf(0L), currentVersions)
      assertEquals(1, fake.renderCount)
      assertEquals(0, fake.lastRenderedPage)
    }

  @Test
  fun surface_detach_releases_buffer_after_scheduler_detaches_surface() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val surface =
        editor.attachSurface(page = 0, handle = 11L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      dispatcher.scheduler.advanceUntilIdle()
      surface.detach { fake.surfaceEvents += "release:11" }

      assertEquals(listOf("attach:0:11"), fake.surfaceEvents)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf("attach:0:11", "detach:0", "release:11"), fake.surfaceEvents)
    }

  @Test
  fun surface_detach_failure_still_releases_buffer() =
    runTest(dispatcher) {
      val failure = IllegalStateException("detach boom")
      val fake = FakeFfiEditor(detachSurfaceProvider = { throw failure })
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      val surface =
        editor.attachSurface(page = 0, handle = 11L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      dispatcher.scheduler.advanceUntilIdle()
      surface.detach { fake.surfaceEvents += "release:11" }

      assertEquals(listOf("attach:0:11"), fake.surfaceEvents)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf<Throwable>(failure), reported)
      assertEquals(listOf("attach:0:11", "release:11"), fake.surfaceEvents)
    }

  @Test
  fun surface_detach_after_editor_dispose_detaches_before_releasing_buffer() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val surface =
        editor.attachSurface(page = 0, handle = 11L, width = 0.0, height = 0.0, scaleFactor = 1.0)

      dispatcher.scheduler.advanceUntilIdle()
      editor.dispose()
      surface.detach { fake.surfaceEvents += "release:11" }

      assertEquals(listOf("attach:0:11"), fake.surfaceEvents)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf("attach:0:11", "detach:0", "release:11"), fake.surfaceEvents)
    }

  @Test
  fun surface_render_failure_settles_failed_page_and_continues_remaining_pages() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val failure = IllegalStateException("render boom")
      val fake =
        FakeFfiEditor(
          onTick = {
            listOf(EditorEvent.StateChanged(listOf(StateField.Cursor)), renderInvalidated())
          },
          cursorProvider = { fakeCursor },
          renderSurfaceProvider = { page ->
            if (page == 0) throw failure
            true
          },
        )
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      val failedSurface =
        editor.attachSurface(page = 0, handle = 10L, width = 0.0, height = 0.0, scaleFactor = 1.0)
      val continuingSurface =
        editor.attachSurface(page = 1, handle = 11L, width = 0.0, height = 0.0, scaleFactor = 1.0)
      val presentedVersions = mutableListOf<Long>()

      var returned = false
      val job =
        launch(dispatcher) {
          editor.await { enqueue(sampleMessage) }
          returned = true
        }
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(returned)

      failedSurface.requestRender { error("failed surface should not present") }
      continuingSurface.requestRender { presentedVersions += it }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf<Throwable>(failure), reported)
      assertEquals(listOf("attach:0:10", "attach:1:11", "render:0", "render:1"), fake.surfaceEvents)
      assertEquals(listOf(1L), presentedVersions)
      editor.onPageSettled(page = 1, version = 1L)
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(returned)
      assertEquals(fakeCursor, editor.cursor)
      job.join()
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
  fun queued_enqueue_holds_local_transaction_barrier_until_tick_commits() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.enqueue(sampleMessage)
      val quiescence = editor.quiesceLocalEdits()
      val barrier = async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }

      assertFalse(barrier.isCompleted)

      dispatcher.scheduler.runCurrent()
      assertTrue(barrier.await().isSuccess)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun queued_enqueue_failure_reaches_local_transaction_barrier() =
    runTest(dispatcher) {
      val boom = RuntimeException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val editor = Editor(fake, this, dispatcher)

      editor.enqueue(sampleMessage)
      val quiescence = editor.quiesceLocalEdits()
      val barrier = async { quiescence.await() }

      dispatcher.scheduler.advanceUntilIdle()

      val result = barrier.await()
      assertTrue(result.isFailure)
      assertEquals(boom.message, result.exceptionOrNull()?.message)
    }

  @Test
  fun enqueue_is_rejected_after_local_transactions_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.quiesceLocalEdits()

      editor.enqueue(sampleMessage)
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
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
