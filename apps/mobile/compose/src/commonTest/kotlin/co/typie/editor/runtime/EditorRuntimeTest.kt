package co.typie.editor.runtime

import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.sync.createTestDocumentEditingSession
import kotlin.test.Test
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorRuntimeTest {
  private fun TestScope.createSession(editor: Editor): DocumentEditingSession =
    createTestDocumentEditingSession(editor, CoroutineScope(coroutineContext))

  @Test
  fun editorOnlyAttachmentHasNoDocumentSession() = runTest {
    val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
    val runtime = EditorRuntime(uiScope = this)

    runtime.attach(editor)

    assertSame(editor, runtime.editor)
    assertNull(runtime.session)
  }

  @Test
  fun attachingDocumentSessionPublishesEditorAndSessionTogether() = runTest {
    val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
    val session = createSession(editor)
    val runtime = EditorRuntime(uiScope = this)

    assertNull(runtime.editor)
    assertNull(runtime.session)

    runtime.attach(session)

    assertSame(editor, runtime.editor)
    assertSame(session, runtime.session)
  }

  @Test
  fun editorOnlyAttachmentCannotBePromotedToDocumentSession() = runTest {
    val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
    val session = createSession(editor)
    val runtime = EditorRuntime(uiScope = this)

    runtime.attach(editor)

    assertFailsWith<IllegalStateException> { runtime.attach(session) }
    assertSame(editor, runtime.editor)
    assertNull(runtime.session)
  }

  @Test
  fun documentSessionCannotBeReplacedUsingTheSameEditor() = runTest {
    val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
    val first = createSession(editor)
    val second = createSession(editor)
    val runtime = EditorRuntime(uiScope = this)

    runtime.attach(first)

    assertFailsWith<IllegalStateException> { runtime.attach(second) }

    assertSame(editor, runtime.editor)
    assertSame(first, runtime.session)
  }

  @Test
  fun staleSessionCannotClearReplacement() = runTest {
    val first = createSession(Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)))
    val second = createSession(Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)))
    val runtime = EditorRuntime(uiScope = this)

    runtime.attach(first)
    runtime.attach(second)
    runtime.clear(first)

    assertSame(second.editor, runtime.editor)
    assertSame(second, runtime.session)

    runtime.clear(second)
    assertNull(runtime.editor)
    assertNull(runtime.session)
  }

  @Test
  fun staleSessionErrorCannotClearReplacement() = runTest {
    val first = createSession(Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)))
    val second = createSession(Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)))
    val runtime = EditorRuntime(uiScope = this)

    runtime.attach(first)
    runtime.reportError(first, IllegalStateException("stale reload failure"))
    runtime.attach(second)
    runCurrent()

    assertSame(second.editor, runtime.editor)
    assertSame(second, runtime.session)
    assertNull(runtime.error)
  }

  @Test
  fun fatalErrorRetainsEditorWithoutKeepingItActive() = runTest {
    val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
    val session = createSession(editor)
    val runtime = EditorRuntime(uiScope = this)
    val failure = IllegalStateException("fatal editor failure")

    runtime.attach(session)
    runtime.reportError(session, failure)
    runCurrent()

    assertSame(failure, runtime.error)
    assertNull(runtime.editor)
    assertNull(runtime.session)
    assertSame(editor, runtime.failedEditor)

    runtime.clearError()

    assertNull(runtime.error)
    assertNull(runtime.failedEditor)
  }
}
