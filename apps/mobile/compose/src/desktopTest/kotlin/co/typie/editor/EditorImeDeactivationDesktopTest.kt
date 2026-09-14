package co.typie.editor

import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking

class EditorImeDeactivationDesktopTest {
  @Test
  fun activationWaitsForASelectionBeingAppliedFromAnInactiveSnapshot() {
    val original = Ime("hello", 0, ImeRange(1, 1), null)
    val moved = original.copy(selection = ImeRange(4, 4))
    val currentIme = AtomicReference(original)
    val pauseRead = AtomicBoolean(false)
    val readPaused = CountDownLatch(1)
    val releaseRead = CountDownLatch(1)
    val activationStarted = CountDownLatch(1)
    val editorDispatcher = Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    val uiExecutor = Executors.newSingleThreadExecutor()
    val scope = CoroutineScope(SupervisorJob() + editorDispatcher)
    val fake =
      FakeFfiEditor(
        onTick = {
          listOf(
            EditorEvent.StateChanged(
              listOf(StateField.Ime, StateField.Selection, StateField.Placeholder)
            )
          )
        },
        selectionProvider = { if (pauseRead.get()) FakeFfiEditor.EmptySelection else null },
        imeProvider = { _, _ -> currentIme.get() },
        placeholderProvider = {
          // readSnapshot has already decided to omit IME, but has not published it.
          if (pauseRead.get()) {
            readPaused.countDown()
            check(releaseRead.await(5, TimeUnit.SECONDS))
          }
          null
        },
      )
    val editor = Editor(fake, scope, editorDispatcher)
    try {
      runBlocking { editor.update { enqueue(Message.System(SystemEvent.Initialize)) } }
      editor.setImeSessionActive(false)
      currentIme.set(moved)
      pauseRead.set(true)
      val update = scope.async { editor.update { enqueue(Message.System(SystemEvent.Initialize)) } }
      assertTrue(readPaused.await(5, TimeUnit.SECONDS))
      val activation =
        uiExecutor.submit<Ime?> {
          editor.setImeSessionActive(true)
          activationStarted.countDown()
          editor.refreshImeSnapshot()
          editor.appliedState.ime
        }
      assertTrue(activationStarted.await(5, TimeUnit.SECONDS))
      assertFailsWith<TimeoutException> { activation.get(100, TimeUnit.MILLISECONDS) }
      releaseRead.countDown()
      assertEquals(moved, activation.get(5, TimeUnit.SECONDS))
      runBlocking { update.await() }
      assertEquals(moved, editor.appliedState.ime)
    } finally {
      releaseRead.countDown()
      editor.dispose()
      scope.cancel()
      editorDispatcher.close()
      uiExecutor.shutdownNow()
    }
  }

  @Test
  fun deactivationDoesNotCommitACompositionRemovedByACompetingTick() {
    val composing =
      Ime(text = "한", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
    val committed = composing.copy(composing = null)
    val currentIme = AtomicReference(composing)
    val tickEntered = CountDownLatch(1)
    val releaseTick = CountDownLatch(1)
    val deactivationStarted = CountDownLatch(1)
    val editorExecutor = Executors.newSingleThreadExecutor()
    val uiExecutor = Executors.newSingleThreadExecutor()
    val editorDispatcher = editorExecutor.asCoroutineDispatcher()
    val scope = CoroutineScope(SupervisorJob() + editorDispatcher)
    val fake =
      FakeFfiEditor(
        onTick = {
          tickEntered.countDown()
          check(releaseTick.await(5, TimeUnit.SECONDS))
          listOf(EditorEvent.StateChanged(listOf(StateField.Ime)))
        },
        imeProvider = { _, _ -> currentIme.get() },
      )
    val editor = Editor(fake, scope, editorDispatcher)

    try {
      editor.setImeSessionActive(true)
      runBlocking { editor.refreshImeSnapshot() }
      currentIme.set(committed)

      val competingTick = scope.async {
        editor.update { enqueue(Message.System(SystemEvent.Initialize)) }
      }
      assertTrue(tickEntered.await(5, TimeUnit.SECONDS))

      val deactivation = uiExecutor.submit {
        deactivationStarted.countDown()
        editor.deactivateImeSession()
      }
      assertTrue(deactivationStarted.await(5, TimeUnit.SECONDS))
      assertFailsWith<TimeoutException> { deactivation.get(100, TimeUnit.MILLISECONDS) }

      releaseTick.countDown()
      deactivation.get(5, TimeUnit.SECONDS)
      runBlocking { competingTick.await() }

      assertTrue(
        fake.enqueued.filterIsInstance<Message.TextInput>().none {
          it.ops == listOf(FlatImeOp.CommitAsIs)
        }
      )
    } finally {
      releaseTick.countDown()
      editor.dispose()
      scope.cancel()
      editorDispatcher.close()
      uiExecutor.shutdownNow()
    }
  }
}
