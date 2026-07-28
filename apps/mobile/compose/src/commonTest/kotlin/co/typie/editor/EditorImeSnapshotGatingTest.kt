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
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
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
          .apply { tickWhenIdle = true }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }

      assertEquals(0, imeCalls)
      assertNull(editor.appliedState.ime)
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
          .apply { tickWhenIdle = true }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(0, imeCalls)

      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()

      assertEquals(1, imeCalls)
      assertEquals(ime, editor.appliedState.ime)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `first tick initializes ime and later ticks follow the ime field`() = runTest {
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
          .apply { tickWhenIdle = true }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(1, imeCalls)

      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(2, imeCalls)

      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(2, imeCalls)

      events = listOf(EditorEvent.StateChanged(fields = listOf(StateField.Ime)))
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      assertEquals(3, imeCalls)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `deactivation commits a live composition`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val composingIme =
        Ime(text = "한", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
      val fake = FakeFfiEditor(imeProvider = { _, _ -> composingIme }).apply { tickWhenIdle = true }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(composingIme, editor.appliedState.ime)

      editor.deactivateImeSession()

      assertTrue(
        fake.enqueued.filterIsInstance<Message.TextInput>().any {
          it.ops == listOf(FlatImeOp.CommitAsIs)
        },
        "a live composition must be committed as part of deactivation",
      )
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `deactivation contains a commit failure already owned by the editor`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val failure = IllegalStateException("commit failed")
      val composingIme =
        Ime(text = "한", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
      val reported = mutableListOf<Throwable>()
      val fake =
        FakeFfiEditor(onTick = { throw failure }, imeProvider = { _, _ -> composingIme }).apply {
          tickWhenIdle = true
        }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher, onError = { _, error -> reported += error })
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()

      editor.deactivateImeSession()

      assertTrue(editor.terminal)
      assertTrue(reported.single() === failure)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }

  @Test
  fun `deactivation without a composition does not dispatch a commit`() = runTest {
    Dispatchers.setMain(StandardTestDispatcher(testScheduler))
    try {
      val fake = FakeFfiEditor(imeProvider = { _, _ -> ime }).apply { tickWhenIdle = true }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()

      editor.deactivateImeSession()

      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.TextInput>())
      assertEquals(0, fake.tickCount)

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
          .apply { tickWhenIdle = true }
      val dispatcher = StandardTestDispatcher(testScheduler)
      val scope = CoroutineScope(SupervisorJob() + dispatcher)
      val editor = Editor(fake, scope, dispatcher)
      editor.setImeSessionActive(true)
      editor.refreshImeSnapshot()
      assertEquals(ime, editor.appliedState.ime)

      editor.setImeSessionActive(false)
      editor.update { enqueue(Message.System(SystemEvent.Initialize)) }

      assertNull(editor.appliedState.ime)
      assertEquals(1, imeCalls)
      scope.cancel()
    } finally {
      Dispatchers.resetMain()
    }
  }
}
