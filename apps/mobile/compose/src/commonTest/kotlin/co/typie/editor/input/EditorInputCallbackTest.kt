package co.typie.editor.input

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import kotlin.test.Test
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

class EditorInputCallbackTest {
  private val message = Message.System(SystemEvent.Initialize)

  @Test
  fun `editor callback does not invoke the block after the editor is terminal`() = runTest {
    val editor = Editor(FakeFfiEditor(), this)
    var invoked = false
    editor.dispose()

    val result =
      editor.runCallback<Unit> {
        invoked = true
        error("must not run")
      }

    assertNull(result)
    assertFalse(invoked)
  }

  @Test
  fun `editor callback contains a failure already owned by the editor`() = runTest {
    val failure = IllegalStateException("tick failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(onTick = { throw failure }),
        scope = this,
        onError = { _, error -> reported += error },
      )

    val result = editor.runCallback { editor.updateNow { enqueue(message) } }

    assertNull(result)
    assertTrue(editor.terminal)
    assertSame(failure, reported.single())
  }

  @Test
  fun `editor callback contains a direct enqueue failure after routing it to the editor`() =
    runTest {
      val failure = IllegalStateException("admission failed")
      val reported = mutableListOf<Throwable>()
      val editor =
        Editor(
          inner = FakeFfiEditor(beforeEnqueueRequest = { throw failure }),
          scope = this,
          onError = { _, error -> reported += error },
        )

      val result = editor.runCallback { editor.enqueue(message) }

      assertNull(result)
      assertTrue(editor.terminal)
      assertSame(failure, reported.single())
    }

  @Test
  fun `editor callback rethrows a nonterminal programming error`() = runTest {
    val failure = IllegalStateException("programming error")
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<IllegalStateException> { editor.runCallback<Unit> { throw failure } }

    assertSame(failure, thrown)
    assertFalse(editor.terminal)
  }

  @Test
  fun `editor callback rethrows an unrelated error after the editor fails`() = runTest {
    val editorFailure = IllegalStateException("editor failed")
    val programmingError = IllegalArgumentException("programming error")
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<IllegalArgumentException> {
        editor.runCallback<Unit> {
          editor.fail(editorFailure)
          throw programmingError
        }
      }

    assertSame(programmingError, thrown)
    assertTrue(editor.terminal)
  }

  @Test
  fun `editor callback never contains cancellation`() = runTest {
    val cancellation = CancellationException("cancelled")
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<CancellationException> { editor.runCallback<Unit> { throw cancellation } }

    assertSame(cancellation, thrown)
    assertFalse(editor.terminal)
  }

  @Test
  fun `editor effect rethrows an unrelated error after the editor fails`() = runTest {
    val editorFailure = IllegalStateException("editor failed")
    val programmingError = IllegalArgumentException("programming error")
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<IllegalArgumentException> {
        editor.runEffect {
          editor.fail(editorFailure)
          throw programmingError
        }
      }

    assertSame(programmingError, thrown)
    assertTrue(editor.terminal)
  }

  @Test
  fun `editor effect reports incomplete when the editor becomes terminal without throwing`() =
    runTest {
      val failure = IllegalStateException("editor failed")
      val editor = Editor(FakeFfiEditor(), this)

      val completed = editor.runEffect { editor.fail(failure) }

      assertFalse(completed)
      assertTrue(editor.terminal)
    }

  @Test
  fun `editor effect rethrows an unrelated wrapper around the editor failure`() = runTest {
    val editorFailure = IllegalStateException("editor failed")
    val wrapper = RuntimeException("programming error", editorFailure)
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<RuntimeException> {
        editor.runEffect {
          editor.fail(editorFailure)
          throw wrapper
        }
      }

    assertSame(wrapper, thrown)
    assertTrue(editor.terminal)
  }

  @Test
  fun `editor effect rethrows a lookalike wrapper around the editor failure`() = runTest {
    val editorFailure = IllegalStateException("editor failed")
    val wrapper = IllegalStateException("editor failed", editorFailure)
    val editor = Editor(FakeFfiEditor(), this)

    val thrown =
      assertFailsWith<IllegalStateException> {
        editor.runEffect {
          editor.fail(editorFailure)
          throw wrapper
        }
      }

    assertSame(wrapper, thrown)
    assertTrue(editor.terminal)
  }

  @Test
  fun `editor effect stops when a business read fails`() = runTest {
    val failure = IllegalStateException("prose read failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(proseTextAnnotatedProvider = { throw failure }),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )
    var continued = false

    val completed = editor.runEffect {
      editor.proseTextAnnotated()
      continued = true
    }

    assertFalse(completed)
    assertFalse(continued)
    runCurrent()
    assertSame(failure, reported.single())
  }

  @Test
  fun `editor effect contains a terminal failure observed by a later business read`() = runTest {
    val failure = IllegalStateException("editor failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )
    val started = CompletableDeferred<Unit>()
    val proceed = CompletableDeferred<Unit>()

    val effect =
      async(start = CoroutineStart.UNDISPATCHED) {
        editor.runEffect {
          started.complete(Unit)
          proceed.await()
          editor.proseTextAnnotated()
        }
      }
    started.await()
    editor.fail(failure)
    proceed.complete(Unit)
    runCurrent()

    assertFalse(effect.await())
    assertSame(failure, reported.single())
  }

  @Test
  fun `editor effect contains a terminal failure observed before update admission`() = runTest {
    val failure = IllegalStateException("editor failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )

    val effect =
      async(start = CoroutineStart.UNDISPATCHED) {
        editor.runEffect { editor.update { enqueue(message) } }
      }
    editor.fail(failure)
    runCurrent()

    assertFalse(effect.await())
    assertSame(failure, reported.single())
  }

  @Test
  fun `editor effect contains its failure across nested deferred awaits`() = runTest {
    val failure = IllegalStateException("nested update failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(beforeEnqueueRequest = { throw failure }),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )

    val completed = editor.runEffect {
      coroutineScope { async { async { editor.update { enqueue(message) } }.await() }.await() }
    }

    assertFalse(completed)
    runCurrent()
    assertSame(failure, reported.single())
  }

  @Test
  fun `editor effect never contains cancellation`() = runTest {
    val cancellation = CancellationException("cancelled")
    val editor = Editor(FakeFfiEditor(), this)

    val thrown = assertFailsWith<CancellationException> { editor.runEffect { throw cancellation } }

    assertSame(cancellation, thrown)
    assertFalse(editor.terminal)
  }
}
