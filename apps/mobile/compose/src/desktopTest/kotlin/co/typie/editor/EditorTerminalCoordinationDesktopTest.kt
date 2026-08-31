package co.typie.editor

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ResourceUpdate
import co.typie.editor.ffi.SystemEvent
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.supervisorScope
import kotlinx.coroutines.withTimeout

class EditorTerminalCoordinationDesktopTest {
  private class FakeResourceUpdate : ResourceUpdate

  private val message = Message.System(SystemEvent.Initialize)

  @Test
  fun disposeCannotMissARequestReceiptBeingRegistered() {
    val failure = terminalCannotMissRequestReceipt { editor -> editor.dispose() }

    assertIs<CancellationException>(failure)
  }

  @Test
  fun failureCannotMissARequestReceiptBeingRegistered() {
    val expected = IllegalStateException("resource admission failed")
    val failure =
      terminalCannotMissRequestReceipt(expected) { editor ->
        runCatching { editor.receiveResourceUpdate(FakeResourceUpdate()) }
      }

    // 코어 실패는 EditorFailureSignal 로 감싸 전달된다(34e5732ee) — 프로덕션이 함께 제공하는
    // unwrapEditorFailureSignal 로 원래 실패를 꺼내 검증한다.
    val unwrapped = failure.unwrapEditorFailureSignal()
    assertIs<IllegalStateException>(unwrapped)
    assertEquals(expected.message, unwrapped.message)
  }

  private fun terminalCannotMissRequestReceipt(
    terminalFailure: Throwable? = null,
    terminate: (Editor) -> Unit,
  ): Throwable = runBlocking {
    supervisorScope {
      val enqueueStarted = CountDownLatch(1)
      val releaseEnqueue = CountDownLatch(1)
      val fake =
        FakeFfiEditor(
          receiveResourceUpdateProvider = { terminalFailure?.let { throw it } },
          beforeEnqueueRequest = {
            enqueueStarted.countDown()
            check(releaseEnqueue.await(5, TimeUnit.SECONDS))
          },
        )
      val editor = Editor(fake, this, Dispatchers.Default)
      val update = async(Dispatchers.Default) { editor.update { enqueue(message) } }
      assertTrue(enqueueStarted.await(5, TimeUnit.SECONDS))

      val terminal = async(Dispatchers.Default) { terminate(editor) }
      withTimeout(5_000) {
        while (!editor.terminal) {
          kotlinx.coroutines.yield()
        }
      }
      releaseEnqueue.countDown()
      terminal.await()

      val failure = withTimeout(5_000) { runCatching { update.await() }.exceptionOrNull() }
      requireNotNull(failure)
    }
  }
}
