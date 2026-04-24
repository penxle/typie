package co.typie.editor

import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.SystemEvent
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
      val editor = Editor(fake, this)

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
      val editor = Editor(fake, this)

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
      val editor = Editor(fake, this)

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
      val editor = Editor(fake, this)

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
      val editor = Editor(fake, this)

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
}
