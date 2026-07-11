package co.typie.screen.editor.editor.toolbar.contextual

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class AttachmentOperationTest {
  @Test
  fun imageAndFilePendingRemainUntilLocalCommitCompletes() = runTest {
    for (kind in listOf(AttachmentKind.Image, AttachmentKind.File)) {
      var pending = true
      var cached = false
      val allowCommit = CompletableDeferred<Unit>()

      val operation = async {
        completeAttachmentOperation(
          persist = { "$kind-asset" },
          cache = { cached = true },
          commit = { allowCommit.await() },
          clearPending = { pending = false },
        )
      }
      runCurrent()

      assertTrue(cached, "$kind metadata should be cached before commit")
      assertTrue(pending, "$kind pending should remain during commit")
      assertFalse(operation.isCompleted)

      allowCommit.complete(Unit)
      assertEquals("$kind-asset", operation.await())
      assertFalse(pending, "$kind pending should clear after commit")
    }
  }

  @Test
  fun imageAndFileCommitFailureDoesNotReportSuccessAndClearsPending() = runTest {
    for (kind in listOf(AttachmentKind.Image, AttachmentKind.File)) {
      var pending = true
      val commitFailure = IllegalStateException("$kind commit failed")

      val failure =
        assertFailsWith<AttachmentException> {
          completeAttachmentOperation(
            persist = { "$kind-asset" },
            cache = {},
            commit = { throw commitFailure },
            clearPending = { pending = false },
          )
        }

      assertEquals(AttachmentFailureStage.CommitDocument, failure.stage)
      assertSame(commitFailure, failure.cause)
      assertFalse(pending)
    }
  }

  @Test
  fun persistenceFailureIsClassifiedAndClearsPending() = runTest {
    var pending = true
    val persistFailure = IllegalStateException("persist failed")

    val failure =
      assertFailsWith<AttachmentException> {
        completeAttachmentOperation<String>(
          persist = { throw persistFailure },
          cache = {},
          commit = {},
          clearPending = { pending = false },
        )
      }

    assertEquals(AttachmentFailureStage.PersistAsset, failure.stage)
    assertSame(persistFailure, failure.cause)
    assertFalse(pending)
  }

  @Test
  fun cancellationPassesThroughWithoutBecomingAttachmentFailure() = runTest {
    var pending = true
    val cancellation = CancellationException("cancelled")

    val thrown =
      assertFailsWith<CancellationException> {
        completeAttachmentOperation<String>(
          persist = { throw cancellation },
          cache = {},
          commit = {},
          clearPending = { pending = false },
        )
      }

    assertSame(cancellation, thrown)
    assertTrue(pending)
  }
}
