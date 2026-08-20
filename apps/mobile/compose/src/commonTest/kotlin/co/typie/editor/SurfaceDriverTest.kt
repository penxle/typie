package co.typie.editor

import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Revision
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalAtomicApi::class)
class SurfaceDriverTest {
  private val dispatcher = StandardTestDispatcher()
  private val configuration = SurfaceConfiguration(100.0, 200.0, 2.0)

  @Test
  fun preservesNativeOperationOrderAndExactRenderIdentity() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val failures = mutableListOf<Throwable>()
      val driver = driver(fake, failures)
      val session = driver.attach(editor, page = 2, handle = 12L, configuration = configuration)
      advanceUntilIdle()

      val resized = SurfaceConfiguration(120.0, 240.0, 3.0)
      var rendered: FrameKey? = null
      driver.resize(session, resized)
      driver.render(session, revision = 9L) { rendered = it }
      advanceUntilIdle()
      driver.detach(session) { fake.surfaceEvents += "complete:2" }
      advanceUntilIdle()

      assertEquals(
        listOf("attach:2:12", "resize:2:120.0:240.0:3.0", "render:2", "detach:2", "complete:2"),
        fake.surfaceEvents,
      )
      assertEquals(Revision(9L), fake.renderCalls.single().requestedRevision)
      assertEquals(FrameKey(1L), rendered)
      assertEquals(emptyList(), failures)
    }

  @Test
  fun ignoresCommandsFromAReplacedSession() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val driver = driver(fake)
      val stale = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      advanceUntilIdle()
      val current = driver.attach(editor, page = 0, handle = 2L, configuration = configuration)
      advanceUntilIdle()

      driver.resize(stale, SurfaceConfiguration(1.0, 1.0, 1.0))
      driver.render(stale, revision = 1L) {}
      driver.render(current, revision = 2L) {}
      advanceUntilIdle()

      assertEquals(emptyList(), fake.resizeCalls)
      assertEquals(listOf(Revision(2L)), fake.renderCalls.map { it.requestedRevision })
    }

  @Test
  fun detachFailureCompletesOnceWithoutFailingTheDriver() =
    runTest(dispatcher) {
      val failure = IllegalStateException("detach failed")
      val fake = FakeFfiEditor(detachSurfaceProvider = { throw failure })
      val editor = Editor(fake, this, dispatcher)
      val failures = mutableListOf<Throwable>()
      val driver = driver(fake, failures)
      val session = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      advanceUntilIdle()
      var completions = 0

      driver.detach(session) { completions += 1 }
      advanceUntilIdle()

      assertEquals(1, completions)
      assertEquals(emptyList(), failures)
    }

  @Test
  fun renderFailureCompletesOnceWithoutInventingAFrame() =
    runTest(dispatcher) {
      val failure = IllegalStateException("render failed")
      val fake = FakeFfiEditor(renderSurfaceProvider = { throw failure })
      val editor = Editor(fake, this, dispatcher)
      val failures = mutableListOf<Throwable>()
      val driver = driver(fake, failures)
      val session = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      advanceUntilIdle()
      val completions = mutableListOf<FrameKey?>()

      driver.render(session, revision = 1L) { completions += it }
      advanceUntilIdle()

      assertEquals(listOf<Throwable>(failure), failures)
      assertEquals(listOf<FrameKey?>(null), completions)
    }

  @Test
  fun disposeCompletesQueuedRenderOnce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val disposed = AtomicBoolean(false)
      val driver = driver(fake, disposed = disposed)
      val session = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      advanceUntilIdle()
      val completions = mutableListOf<FrameKey?>()

      driver.render(session, revision = 1L) { completions += it }
      disposed.store(true)
      driver.dispose()
      advanceUntilIdle()

      assertEquals(listOf<FrameKey?>(null), completions)
      assertNull(fake.renderCalls.singleOrNull())
    }

  @Test
  fun failureRejectsNewSurfaceWorkButStillAllowsDetach() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val failure = AtomicReference<Throwable?>(null)
      val driver = driver(fake, failure = failure)
      val session = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      advanceUntilIdle()
      val completions = mutableListOf<FrameKey?>()

      failure.store(IllegalStateException("editor failed"))
      driver.resize(session, SurfaceConfiguration(1.0, 1.0, 1.0))
      driver.render(session, revision = 1L) { completions += it }
      driver.detach(session)
      advanceUntilIdle()

      assertEquals(listOf<FrameKey?>(null), completions)
      assertEquals(emptyList(), fake.resizeCalls)
      assertEquals(emptyList(), fake.renderCalls)
      assertEquals(listOf("attach:0:1", "detach:0"), fake.surfaceEvents)
    }

  @Test
  fun cancelledOwnerScopeStillCompletesDetachOnce() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val owner = SupervisorJob()
      val driver = driver(fake, scope = CoroutineScope(owner + dispatcher))
      val session = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      advanceUntilIdle()
      var completions = 0

      owner.cancel()
      driver.detach(session) { completions += 1 }
      advanceUntilIdle()

      assertEquals(1, completions)
      assertEquals(listOf("attach:0:1", "detach:0"), fake.surfaceEvents)
    }

  @Test
  fun commandCancellationDoesNotSkipLaterDetachCompletion() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor(
          detachSurfaceProvider = { page ->
            if (page == 0) throw CancellationException("first detach cancelled")
          }
        )
      val editor = Editor(fake, this, dispatcher)
      val driver = driver(fake)
      val first = driver.attach(editor, page = 0, handle = 1L, configuration = configuration)
      val second = driver.attach(editor, page = 1, handle = 2L, configuration = configuration)
      advanceUntilIdle()
      val completed = mutableListOf<Int>()

      driver.detach(first) { completed += 0 }
      driver.detach(second) { completed += 1 }
      advanceUntilIdle()

      assertEquals(listOf(0, 1), completed)
      assertEquals(listOf("attach:0:1", "attach:1:2", "detach:1"), fake.surfaceEvents)
    }

  private fun driver(
    fake: FakeFfiEditor,
    failures: MutableList<Throwable> = mutableListOf(),
    disposed: AtomicBoolean = AtomicBoolean(false),
    failure: AtomicReference<Throwable?> = AtomicReference(null),
    scope: CoroutineScope = CoroutineScope(dispatcher),
  ): SurfaceDriver =
    SurfaceDriver(
      inner = fake,
      scope = scope,
      dispatcher = dispatcher,
      disposed = disposed,
      failure = failure,
      notifyFailure = { failures += it },
    )
}
