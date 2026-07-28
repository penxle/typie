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

private val sampleMessage: Message = Message.System(SystemEvent.Initialize)

private fun publicEvent(): EditorEvent = EditorEvent.CursorExitedDocumentStart

private fun proseRange(
  id: String,
  group: String,
  start: Int,
  end: Int,
): ProseTrackedRangeRegistration =
  ProseTrackedRangeRegistration(id = id, group = group, start = start, end = end)

@OptIn(ExperimentalCoroutinesApi::class)
class EditorRequestTest {
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
  fun initial_applied_and_published_states_are_EditorState_Initial() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      assertFalse(editor.terminal)
      assertEquals(EditorState.Initial, editor.appliedState)
      assertEquals(EditorState.Initial, editor.publishedState)
      assertEquals(null, editor.appliedState.cursor)
      assertEquals(null, editor.appliedState.selection)
      assertEquals(emptyList(), editor.appliedState.pageSizes)
      assertEquals(null, editor.appliedState.rootAttrs)
      assertEquals(null, editor.appliedState.ime)
    }

  @Test
  fun update_enqueues_messages_and_ticks_once() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.update { enqueue(sampleMessage) }

      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun update_afterApplied_exception_does_not_fail_editor() =
    runTest(dispatcher) {
      val boom = IllegalStateException("boom")
      val reported = mutableListOf<Throwable>()
      val editor =
        Editor(FakeFfiEditor(), this, dispatcher, onError = { _, error -> reported += error })

      val thrown =
        assertFailsWith<IllegalStateException> {
          editor.update(afterApplied = { throw boom }) { enqueue(sampleMessage) }
        }

      assertEquals(boom.message, thrown.message)
      assertFalse(editor.terminal)
      assertTrue(reported.isEmpty())
      assertEquals(1L, editor.appliedState.version)
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
      assertEquals(0L, editor.appliedState.version)
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
      assertEquals(1L, editor.appliedState.version)
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
  fun empty_update_block_does_not_tick() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.update { /* nothing */ }

      assertEquals(0, fake.tickCount)
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun update_applies_state_from_tick() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 2f, 3f, 4f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.update { enqueue(sampleMessage) }

      assertEquals(fakeCursor, editor.appliedState.cursor)
      assertEquals(1L, editor.appliedState.version)
    }

  @Test
  fun update_applies_last_history_tag_from_state_change() =
    runTest(dispatcher) {
      val tag = HistoryTag.PasteHtml(plainText = "hello", start = 3)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.LastHistoryTag))) },
          lastHistoryTagProvider = { tag },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.update { enqueue(sampleMessage) }

      assertEquals(tag, editor.appliedState.lastHistoryTag)
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

      editor.update { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(emptyList(), reported)
      assertEquals(null, editor.appliedState.selection)
      assertEquals(null, editor.appliedState.ime)
      assertEquals(1, cursorExited)
    }

  @Test
  fun update_returns_applied_snapshot_before_visual_publication() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 2f, 3f, 4f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      val update = requireNotNull(editor.update { enqueue(sampleMessage) })

      assertEquals(1L, update.snapshot.version)
      assertEquals(fakeCursor, update.snapshot.cursor)
      assertEquals(0L, editor.publishedState.version)
      assertEquals(1L, editor.appliedState.version)
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

      editor.update { enqueue(sampleMessage) }

      assertEquals(1, fake.trackedRangesCallCount)
      assertEquals(1, fake.trackedRangesContainingPositionCallCount)
      assertEquals(listOf(range), editor.appliedState.trackedRanges)
      assertEquals(listOf(rangeEndpoints), editor.appliedState.trackedRangesContainingSelectionHead)

      editor.update { enqueue(sampleMessage) }

      assertEquals(1, fake.trackedRangesCallCount)
      assertEquals(1, fake.trackedRangesContainingPositionCallCount)
      assertEquals(listOf(range), editor.appliedState.trackedRanges)
      assertEquals(listOf(rangeEndpoints), editor.appliedState.trackedRangesContainingSelectionHead)
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

      editor.update { enqueue(sampleMessage) }

      assertEquals(1, fake.placeholderCallCount)
      assertEquals(placeholder, editor.appliedState.placeholder)

      currentPlaceholder = laterPlaceholder
      editor.update { enqueue(sampleMessage) }

      assertEquals(1, fake.placeholderCallCount)
      assertEquals(placeholder, editor.appliedState.placeholder)
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

      editor.update { enqueue(sampleMessage) }
      assertEquals(1L, editor.appliedState.version)
      assertEquals(1L, editor.appliedState.documentRevision)

      // cursor-only tick: version advances, documentRevision stays
      editor.update { enqueue(sampleMessage) }
      assertEquals(2L, editor.appliedState.version)
      assertEquals(1L, editor.appliedState.documentRevision)

      editor.update { enqueue(sampleMessage) }
      assertEquals(3L, editor.appliedState.version)
      assertEquals(2L, editor.appliedState.documentRevision)
    }

  @Test
  fun update_reports_and_propagates_tick_exception_without_applying() =
    runTest(dispatcher) {
      val boom = RuntimeException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      val thrown = assertFailsWith<RuntimeException> { editor.update { enqueue(sampleMessage) } }

      assertEquals(boom.message, thrown.message)
      assertEquals(1, reported.size)
      assertTrue(reported.single() === boom)
      assertEquals(EditorState.Initial, editor.appliedState)
    }

  @Test
  fun update_is_rejected_after_local_transactions_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.quiesceLocalEdits()

      assertEquals(null, editor.update { enqueue(sampleMessage) })

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
  fun updateNow_enqueues_ticks_and_applies_inline() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(5f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.updateNow { enqueue(sampleMessage) }

      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
      assertEquals(fakeCursor, editor.appliedState.cursor)
      assertEquals(1L, editor.appliedState.version)
    }

  @Test
  fun updateNow_is_rejected_after_local_transactions_quiesce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      editor.quiesceLocalEdits()

      editor.updateNow { enqueue(sampleMessage) }

      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
    }

  @Test
  fun updateNow_rejected_by_admission_does_not_enqueue_or_tick() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val update = editor.updateNow(admit = { false }) { enqueue(sampleMessage) }

      assertEquals(null, update)
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
      assertEquals(fakeCursor, editor.appliedState.cursor)
      assertEquals(1L, editor.appliedState.version)
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
  fun updateNow_reports_tick_exception_without_applying() =
    runTest(dispatcher) {
      val boom = IllegalStateException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      val thrown =
        assertFailsWith<IllegalStateException> { editor.updateNow { enqueue(sampleMessage) } }

      assertTrue(thrown === boom)
      assertEquals(1, reported.size)
      assertTrue(reported.single() === boom)
      assertEquals(EditorState.Initial, editor.appliedState)
      assertTrue(editor.terminal)

      val enqueuedAfterFailure = fake.enqueued.toList()
      var updateBuilt = false
      var updateNowBuilt = false
      var localEditStarted = false

      assertEquals(
        null,
        editor.update {
          updateBuilt = true
          enqueue(sampleMessage)
        },
      )
      assertEquals(
        null,
        editor.updateNow {
          updateNowBuilt = true
          enqueue(sampleMessage)
        },
      )
      editor.enqueue(sampleMessage)
      assertEquals(
        null,
        editor.trackLocalEdit { context -> launch(context) { localEditStarted = true } },
      )
      dispatcher.scheduler.advanceUntilIdle()

      assertFalse(updateBuilt)
      assertFalse(updateNowBuilt)
      assertFalse(localEditStarted)
      assertEquals(enqueuedAfterFailure, fake.enqueued)
      assertEquals(1, reported.size)
    }

  @Test
  fun updateNow_applies_consecutive_revisions() =
    runTest(dispatcher) {
      val cursorA =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { cursorA },
        )
      val editor = Editor(fake, this, dispatcher)

      editor.updateNow { enqueue(sampleMessage) }
      assertEquals(1L, editor.appliedState.version)
      editor.updateNow { enqueue(sampleMessage) }
      assertEquals(2L, editor.appliedState.version)
    }

  @Test
  fun listener_receives_event_on_main_dispatcher() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(onTick = { listOf(publicEvent()) })
      val editor = Editor(fake, this, dispatcher)

      val received = mutableListOf<EditorEvent>()
      editor.on<EditorEvent.CursorExitedDocumentStart> { _, e -> received += e }

      editor.update { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(1, received.size)
      assertTrue(received[0] is EditorEvent.CursorExitedDocumentStart)
    }

  @Test
  fun state_changed_events_are_not_delivered_to_listeners() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) })
      val editor = Editor(fake, this, dispatcher)

      var seen = 0
      editor.on<EditorEvent.StateChanged> { _, _ -> seen += 1 }

      editor.update { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(0, seen)
    }

  @Test
  fun unregister_closure_prevents_future_calls() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(onTick = { listOf(publicEvent()) })
      val editor = Editor(fake, this, dispatcher)

      var count = 0
      val off = editor.on<EditorEvent.CursorExitedDocumentStart> { _, _ -> count += 1 }
      editor.update { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(1, count)

      off()
      editor.update { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(1, count)
    }

  @Test
  fun listener_exception_does_not_block_other_listeners() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor(onTick = { listOf(publicEvent()) })
      val editor = Editor(fake, this, dispatcher)

      var second = 0
      editor.on<EditorEvent.CursorExitedDocumentStart> { _, _ -> error("first boom") }
      editor.on<EditorEvent.CursorExitedDocumentStart> { _, _ -> second += 1 }

      editor.update { enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(1, second)
    }

  @Test
  fun update_after_dispose_returns_null_without_building_or_admission() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      var built = false
      editor.dispose()

      assertTrue(editor.terminal)
      assertEquals(
        null,
        editor.update {
          built = true
          enqueue(sampleMessage)
        },
      )
      assertFalse(built)
      assertEquals(emptyList(), fake.enqueued)
      assertEquals(emptyList(), reported)
    }

  @Test
  fun updateNow_after_dispose_returns_null_without_building_or_admission() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      var built = false
      editor.dispose()

      assertTrue(editor.terminal)
      assertEquals(
        null,
        editor.updateNow {
          built = true
          enqueue(sampleMessage)
        },
      )

      assertFalse(built)
      assertEquals(emptyList(), fake.enqueued)
      assertEquals(emptyList(), reported)
    }

  @Test
  fun track_local_edit_after_dispose_returns_null_without_starting() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      var started = false
      editor.dispose()

      assertEquals(null, editor.trackLocalEdit { context -> launch(context) { started = true } })
      dispatcher.scheduler.advanceUntilIdle()

      assertFalse(started)
    }

  @Test
  fun reentrant_updateNow_throws_before_enqueue() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val reported = mutableListOf<Throwable>()
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })

      val error =
        assertFailsWith<IllegalStateException> {
          editor.updateNow {
            enqueue(sampleMessage)
            editor.updateNow { enqueue(sampleMessage) }
          }
        }

      assertEquals("nested updateNow is not supported", error.message)
      assertEquals(emptyList(), reported)
      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
    }

  @Test
  fun reentrant_updateNow_throws_after_the_outer_builder_disposes_the_editor() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)

      val error =
        assertFailsWith<IllegalStateException> {
          editor.updateNow {
            editor.dispose()
            editor.updateNow {}
          }
        }

      assertEquals("nested updateNow is not supported", error.message)
      assertTrue(editor.terminal)
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
  fun update_cancelled_before_resume_keeps_native_and_applied_state_in_sync() =
    runTest(dispatcher) {
      val fakeCursor =
        CursorMetrics(pageIdx = 0, caret = Rect(1f, 0f, 0f, 0f), line = Rect(0f, 0f, 0f, 0f))
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.Cursor))) },
          cursorProvider = { fakeCursor },
        )
      val editor = Editor(fake, this, dispatcher)

      val job = launch(dispatcher) { editor.update { enqueue(sampleMessage) } }
      job.cancel()
      dispatcher.scheduler.advanceUntilIdle()
      job.join()

      if (fake.tickCount > 0) {
        assertEquals(1, fake.tickCount)
        assertEquals(1L, editor.appliedState.version)
        assertEquals(fakeCursor, editor.appliedState.cursor)
      } else {
        assertEquals(EditorState.Initial, editor.appliedState)
      }
    }
}
