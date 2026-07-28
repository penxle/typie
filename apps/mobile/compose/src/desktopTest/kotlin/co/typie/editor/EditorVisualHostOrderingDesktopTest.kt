package co.typie.editor

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout

class EditorVisualHostOrderingDesktopTest {
  @Test
  fun deactivateThenActivatePreservesCallOrderWhileTickOwnsMutex() = runBlocking {
    val tickEntered = CountDownLatch(1)
    val releaseTick = CountDownLatch(1)
    val editorDispatcher = Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    val uiDispatcher = Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    val editorScope = CoroutineScope(SupervisorJob() + editorDispatcher)
    val editor =
      Editor(
        inner =
          FakeFfiEditor(
            onTick = {
              tickEntered.countDown()
              check(releaseTick.await(5, TimeUnit.SECONDS))
              emptyList()
            }
          ),
        scope = editorScope,
        dispatcher = editorDispatcher,
      )
    val firstToken = Any()
    val secondToken = Any()
    editor.activateVisualHost(firstToken)

    try {
      editor.enqueue(Message.System(SystemEvent.Initialize))
      assertTrue(tickEntered.await(5, TimeUnit.SECONDS))

      editor.deactivateVisualHost(firstToken)
      val activationStarted = CountDownLatch(1)
      val activation =
        async(uiDispatcher) {
          activationStarted.countDown()
          runCatching { editor.activateVisualHost(secondToken) }
        }

      assertTrue(activationStarted.await(5, TimeUnit.SECONDS))
      Thread.sleep(200)
      releaseTick.countDown()

      val result = withTimeout(5_000) { activation.await() }
      assertTrue(
        result.isSuccess,
        "later visual-host activation overtook deactivation: ${result.exceptionOrNull()}",
      )
    } finally {
      releaseTick.countDown()
      editorScope.cancel()
      editorDispatcher.close()
      uiDispatcher.close()
    }
  }
}
