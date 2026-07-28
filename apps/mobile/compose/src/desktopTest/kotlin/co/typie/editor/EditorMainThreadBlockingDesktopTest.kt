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
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull

class EditorMainThreadBlockingDesktopTest {
  @Test
  fun retainedFrameReadDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor ->
      suspend { editor.retainedFrames(page = 0) }
    }
  }

  @Test
  fun noOpSurfaceResizeDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor ->
      val configuration = SurfaceConfiguration(width = 100.0, height = 100.0, scaleFactor = 1.0)
      editor.activateVisualHost(Unit)
      val session =
        editor.attachSurface(
          page = 0,
          handle = 1L,
          width = configuration.width,
          height = configuration.height,
          scaleFactor = configuration.scaleFactor,
          wakeDelivery = {},
        )
      suspend { session.requestResize(configuration) }
    }
  }

  @Test
  fun visualHostDeactivationDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor ->
      val token = Any()
      editor.activateVisualHost(token)
      suspend { editor.deactivateVisualHost(token) }
    }
  }

  @Test
  fun surfaceDetachDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor ->
      editor.activateVisualHost(Unit)
      val session =
        editor.attachSurface(
          page = 0,
          handle = 1L,
          width = 100.0,
          height = 100.0,
          scaleFactor = 1.0,
          wakeDelivery = {},
        )
      suspend { session.detach() }
    }
  }

  @Test
  fun surfaceDeliveryFailureDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor ->
      editor.activateVisualHost(Unit)
      val session =
        editor.attachSurface(
          page = 0,
          handle = 1L,
          width = 100.0,
          height = 100.0,
          scaleFactor = 1.0,
          wakeDelivery = {},
        )
      suspend {
        editor.surfaceDeliveryFailed(
          page = 0,
          session = session,
          error = IllegalStateException("surface delivery failed"),
        )
      }
    }
  }

  @Test
  fun disposeDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor -> suspend { editor.dispose() } }
  }

  @Test
  fun failureDoesNotBlockUiWhileTickOwnsEditorMutex() {
    assertUiRemainsResponsiveWhileTickOwnsEditorMutex { editor ->
      suspend { editor.fail(IllegalStateException("editor failed")) }
    }
  }

  private fun assertUiRemainsResponsiveWhileTickOwnsEditorMutex(
    prepareAction: (Editor) -> suspend () -> Unit
  ) = runBlocking {
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
    val action = prepareAction(editor)
    var responsive = false

    try {
      editor.enqueue(Message.System(SystemEvent.Initialize))
      assertTrue(tickEntered.await(5, TimeUnit.SECONDS))

      val actionStarted = CountDownLatch(1)
      val actionJob =
        launch(uiDispatcher) {
          actionStarted.countDown()
          action()
        }
      assertTrue(actionStarted.await(5, TimeUnit.SECONDS))

      responsive = withTimeoutOrNull(250) { withContext(uiDispatcher) { true } } ?: false

      releaseTick.countDown()
      withTimeout(5_000) { actionJob.join() }
      withTimeout(5_000) { withContext(editorDispatcher) {} }
    } finally {
      releaseTick.countDown()
      editorScope.cancel()
      editorDispatcher.close()
      uiDispatcher.close()
    }

    assertTrue(responsive, "UI dispatcher was blocked while the Editor mutex was held")
  }
}
