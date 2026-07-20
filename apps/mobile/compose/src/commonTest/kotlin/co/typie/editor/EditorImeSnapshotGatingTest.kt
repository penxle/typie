package co.typie.editor

import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

class EditorImeSnapshotGatingTest {
  private val ime =
    Ime(text = "hello", windowStart = 0, selection = ImeRange(2, 2), composing = null)

  @Test
  fun `ime is not materialized without an active ime session`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var imeCalls = 0
      val fake =
        FakeFfiEditor(
          imeProvider = { _, _ ->
            imeCalls += 1
            ime
          }
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }

      assertEquals(0, imeCalls)
      assertNull(editor.tickIme)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `ime session activation refreshes the snapshot`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var imeCalls = 0
      val fake =
        FakeFfiEditor(
          imeProvider = { _, _ ->
            imeCalls += 1
            ime
          }
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(0, imeCalls)

      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()

      assertEquals(1, imeCalls)
      assertEquals(ime, editor.tickIme)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `ime recomputes only when the ime field changes`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var imeCalls = 0
      var events: List<EditorEvent> = emptyList()
      val fake =
        FakeFfiEditor(
          onTick = { events },
          imeProvider = { _, _ ->
            imeCalls += 1
            Ime(text = "hello", windowStart = 0, selection = ImeRange(2, 2), composing = null)
          },
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(1, imeCalls)
      val first = editor.tickIme

      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(1, imeCalls)
      assertSame(first, editor.tickIme)

      events = listOf(EditorEvent.StateChanged(fields = listOf(StateField.Ime)))
      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(2, imeCalls)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `ime session refresh does not clobber a settle-delayed edit commit`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var events: List<EditorEvent> = emptyList()
      val fake = FakeFfiEditor(onTick = { events }, imeProvider = { _, _ -> ime })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.attachSurface(page = 0, handle = 1L, width = 100.0, height = 100.0, scaleFactor = 1.0)
      runCurrent()

      // A document edit whose commit is held back by the surface settle barrier.
      events =
        listOf(
          EditorEvent.StateChanged(fields = listOf(StateField.Doc)),
          EditorEvent.RenderInvalidated,
        )
      val edit = launch { editor.await { enqueue(Message.System(SystemEvent.Initialize)) } }
      runCurrent()
      assertEquals(0L, editor.state.documentRevision)

      // IME session activates while the edit is still waiting for settlement.
      events = emptyList()
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(ime, editor.tickIme)

      editor.onPageSettled(page = 0, version = Long.MAX_VALUE)
      runCurrent()

      assertTrue(edit.isCompleted)
      assertEquals(
        1L,
        editor.state.documentRevision,
        "the settled edit's snapshot must survive an interleaved ime refresh",
      )
      assertEquals(
        ime,
        editor.state.ime,
        "the activation refresh must survive the settled snapshot: pull-based " +
          "platforms (iOS/desktop) and command normalization read state.ime",
      )
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `session refresh publishes a settle-parked ime to committed state`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var events: List<EditorEvent> = emptyList()
      val fake = FakeFfiEditor(onTick = { events }, imeProvider = { _, _ -> ime })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.attachSurface(page = 0, handle = 1L, width = 100.0, height = 100.0, scaleFactor = 1.0)
      runCurrent()

      // The session flag is already up (focus path) when an edit materializes the
      // ime into tickSnapshot but parks its commit behind the settle barrier.
      editor.setImeSessionActive(true)
      events =
        listOf(
          EditorEvent.StateChanged(fields = listOf(StateField.Ime)),
          EditorEvent.RenderInvalidated,
        )
      val edit = launch { editor.await { enqueue(Message.System(SystemEvent.Initialize)) } }
      runCurrent()
      assertEquals(ime, editor.tickIme)
      assertNull(editor.state.ime)

      events = emptyList()
      editor.refreshImeSnapshot()

      assertEquals(
        ime,
        editor.state.ime,
        "session start must not proceed against a committed state whose ime lags the latest tick",
      )

      editor.onPageSettled(page = 0, version = Long.MAX_VALUE)
      runCurrent()
      assertTrue(edit.isCompleted)
      assertEquals(ime, editor.state.ime)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `deactivation commits a live composition before hiding the ime snapshot`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val composingIme =
        Ime(text = "한", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
      val fake = FakeFfiEditor(imeProvider = { _, _ -> composingIme })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(composingIme, editor.tickIme)

      editor.deactivateImeSession()

      assertTrue(
        fake.enqueued.filterIsInstance<Message.TextInput>().any {
          it.ops == listOf(FlatImeOp.CommitAsIs)
        },
        "a live composition must be committed as part of deactivation",
      )
      assertNull(editor.tickIme)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `deactivation without a composition does not dispatch a commit`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val fake = FakeFfiEditor(imeProvider = { _, _ -> ime })
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()

      editor.deactivateImeSession()

      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.TextInput>())
      assertNull(editor.tickIme)

      // Repeated deactivation is a no-op: nothing left to tear down.
      val ticksAfterDeactivation = fake.tickCount
      editor.deactivateImeSession()
      assertEquals(ticksAfterDeactivation, fake.tickCount)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `ime session deactivation clears the snapshot on the next tick`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      var imeCalls = 0
      val fake =
        FakeFfiEditor(
          imeProvider = { _, _ ->
            imeCalls += 1
            ime
          }
        )
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(ime, editor.tickIme)

      editor.setImeSessionActive(false)
      editor.await { enqueue(Message.System(SystemEvent.Initialize)) }

      assertNull(editor.tickIme)
      assertEquals(1, imeCalls)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }
}
