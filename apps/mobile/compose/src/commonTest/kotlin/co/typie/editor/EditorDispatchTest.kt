package co.typie.editor

import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.SystemEvent
import kotlin.coroutines.ContinuationInterceptor
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

private class FakeFfiEditor(var onTick: () -> List<EditorEvent> = { emptyList() }) :
  co.typie.editor.ffi.Editor {
  val enqueued = mutableListOf<Message>()
  var tickCount = 0

  override fun enqueue(message: Message) {
    enqueued += message
  }

  override fun tick(): List<EditorEvent> {
    tickCount += 1
    return onTick()
  }

  override fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
  ) = error("not used")

  override fun detachSurface(page: Int) = error("not used")

  override fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    error("not used")

  override fun renderSurface(page: Int) = error("not used")

  override fun cursor(): CursorMetrics? = error("not used")

  override fun selection(): Selection = error("not used")

  override fun documentAttrs(): DocumentAttrs = error("not used")

  override fun inspectState(options: InspectStateOptions?): String = error("not used")

  override fun inspectStateAsMacro(): String = error("not used")

  override fun pageSizes(): List<Size> = error("not used")

  override fun ime(beforeLimit: Int, afterLimit: Int): Ime = error("not used")
}

private val sampleMessage: Message = Message.System(SystemEvent.Initialize)
private val nextMessage: Message = Message.System(SystemEvent.FontBaseLoaded("Pretendard", 400))

@OptIn(ExperimentalCoroutinesApi::class)
class EditorDispatchTest {
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
  fun `dispatch resumes after tick completes and enqueues the message`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val completed = CompletableDeferred<Unit>()
      launch(Dispatchers.Main.immediate) {
        editor.dispatch(sampleMessage)
        completed.complete(Unit)
      }

      assertEquals(0, fake.tickCount)
      assertFalse(completed.isCompleted)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
      assertTrue(completed.isCompleted)
    }

  @Test
  fun `dispatch batches multiple messages into one tick`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val completed = CompletableDeferred<Unit>()
      launch(Dispatchers.Main.immediate) {
        editor.dispatch(sampleMessage, nextMessage)
        completed.complete(Unit)
      }

      assertEquals(0, fake.tickCount)
      assertFalse(completed.isCompleted)

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf(sampleMessage, nextMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
      assertTrue(completed.isCompleted)
    }

  @Test
  fun `dispatch propagates tick failure as exception`() =
    runTest(dispatcher) {
      val boom = RuntimeException("boom")
      val fake = FakeFfiEditor(onTick = { throw boom })
      val editor = Editor(fake, this, dispatcher)

      val thrown = CompletableDeferred<Throwable>()
      launch(Dispatchers.Main.immediate) {
        try {
          editor.dispatch(sampleMessage)
        } catch (e: Throwable) {
          thrown.complete(e)
        }
      }

      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(thrown.isCompleted)
      val caught = thrown.getCompleted()
      assertIs<RuntimeException>(caught)
      assertEquals("boom", caught.message)
    }

  @Test
  fun `cancelled dispatch still enqueues message and releases continuation`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val job =
        launch(Dispatchers.Main.immediate, start = CoroutineStart.UNDISPATCHED) {
          editor.dispatch(sampleMessage)
        }

      job.cancel()
      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf(sampleMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
      assertTrue(job.isCancelled)
    }

  @Test
  fun `dispose cancels pending dispatch`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val thrown = CompletableDeferred<Throwable>()
      launch(Dispatchers.Main.immediate, start = CoroutineStart.UNDISPATCHED) {
        try {
          editor.dispatch(sampleMessage)
        } catch (e: Throwable) {
          thrown.complete(e)
        }
      }

      editor.dispose()
      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(thrown.isCompleted)
      assertIs<CancellationException>(thrown.getCompleted())
    }

  @Test
  fun `listener fires inside scheduled tick after enqueue`() =
    runTest(dispatcher) {
      val received = mutableListOf<EditorEvent>()
      val testEvent = EditorEvent.RenderInvalidated
      val fake = FakeFfiEditor(onTick = { listOf(testEvent) })
      val editor = Editor(fake, this, dispatcher)

      editor.on<EditorEvent.RenderInvalidated> { _, e -> received += e }

      launch(Dispatchers.Main.immediate) { editor.enqueue(sampleMessage) }

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(1, received.size)
      assertIs<EditorEvent.RenderInvalidated>(received.first())
    }

  @Test
  fun `sync runs tick inline before returning`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      // sync is non-suspend — call directly
      editor.sync {
        enqueue(sampleMessage)
        enqueue(nextMessage)
      }

      // After return, both messages enqueued + tickCount=1
      assertEquals(listOf(sampleMessage, nextMessage), fake.enqueued)
      assertEquals(1, fake.tickCount)
    }

  @Test
  fun `sync waits for in-flight worker tick before running`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      // 1. Normal enqueue → schedules worker tick
      editor.enqueue(sampleMessage)

      // 2. Immediately call sync — must serialize with worker tick via mutex
      editor.sync { enqueue(nextMessage) }

      dispatcher.scheduler.advanceUntilIdle()

      // 1 worker tick + 1 sync inline tick = 2 total ticks
      assertEquals(2, fake.tickCount)
      assertEquals(listOf(sampleMessage, nextMessage), fake.enqueued)
    }

  @Test
  fun `listener unregister during emit does not throw`() =
    runTest(dispatcher) {
      val testEvent = EditorEvent.RenderInvalidated
      val fake = FakeFfiEditor(onTick = { listOf(testEvent) })
      val editor = Editor(fake, this, dispatcher)

      var unregister: (() -> Unit)? = null
      val received = mutableListOf<Int>()
      // First listener: unregisters itself during emit
      unregister =
        editor.on<EditorEvent.RenderInvalidated> { _, _ ->
          received += 1
          unregister?.invoke()
        }
      // Second listener: also receives event
      editor.on<EditorEvent.RenderInvalidated> { _, _ -> received += 2 }

      launch(Dispatchers.Main.immediate) { editor.enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()

      // Both listeners called (immutable snapshot — no race)
      assertEquals(listOf(1, 2), received)

      // Next tick: first listener is gone
      received.clear()
      launch(Dispatchers.Main.immediate) { editor.enqueue(sampleMessage) }
      dispatcher.scheduler.advanceUntilIdle()
      assertEquals(listOf(2), received)
    }

  @Test
  fun `dispatch resumes on caller's dispatcher, not forced to Main`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val resumeContexts = mutableListOf<String>()
      // Launch on `dispatcher` directly (not Main.immediate). After removing withContext wrap,
      // dispatch should resume on the caller's dispatcher, not switch to Main.immediate.
      launch(dispatcher) {
        val before = coroutineContext[ContinuationInterceptor]
        editor.dispatch(sampleMessage)
        val after = coroutineContext[ContinuationInterceptor]
        resumeContexts += if (before === after) "preserved" else "switched"
      }

      dispatcher.scheduler.advanceUntilIdle()

      assertEquals(listOf("preserved"), resumeContexts)
    }

  @Test
  fun `enqueue after dispose is silently ignored`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.dispose()
      // Post-dispose enqueue (e.g. from a background scope that hasn't been cancelled yet)
      editor.enqueue(sampleMessage)

      dispatcher.scheduler.advanceUntilIdle()

      // inner.enqueue must NOT have been called
      assertEquals(0, fake.enqueued.size)
      // tick must NOT have been called
      assertEquals(0, fake.tickCount)
    }

  @Test
  fun `dispatch after dispose immediately cancels caller`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.dispose()

      val thrown = CompletableDeferred<Throwable>()
      launch(Dispatchers.Main.immediate, start = CoroutineStart.UNDISPATCHED) {
        try {
          editor.dispatch(sampleMessage)
        } catch (e: Throwable) {
          thrown.complete(e)
        }
      }

      dispatcher.scheduler.advanceUntilIdle()

      assertTrue(thrown.isCompleted)
      assertIs<CancellationException>(thrown.getCompleted())
      assertEquals(0, fake.enqueued.size)
      assertEquals(0, fake.tickCount)
    }
}
