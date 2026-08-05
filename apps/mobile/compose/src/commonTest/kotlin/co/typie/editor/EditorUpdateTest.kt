package co.typie.editor

import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.CommandRejection
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ResourceUpdate
import co.typie.editor.ffi.SystemEvent
import co.typie.screen.editor.editor.subpane.comments.COMMENT_RANGE_GROUP
import co.typie.screen.editor.editor.subpane.comments.addFrozenComment
import co.typie.serialization.json
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.put

class EditorUpdateTest {
  private class FakeResourceUpdate : ResourceUpdate

  private val dispatcher = StandardTestDispatcher()
  private val message = Message.System(SystemEvent.Initialize)

  @Test
  fun idleTickReturnsNull() {
    assertNull(FakeFfiEditor().tick())
  }

  @Test
  fun coalescedUpdatesReceiveTheirExactRequestOutcomes() =
    runTest(dispatcher) {
      val fake =
        FakeFfiEditor().apply {
          commandOutcomesProvider = { requestId, messages ->
            if (requestId.value == 1L) {
              List(messages.size) { CommandOutcome.Applied }
            } else {
              List(messages.size) { CommandOutcome.Rejected(CommandRejection.InvalidArgument) }
            }
          }
        }
      val editor = Editor(fake, this, dispatcher)

      val first = async(start = CoroutineStart.UNDISPATCHED) { editor.update { enqueue(message) } }
      val second = async(start = CoroutineStart.UNDISPATCHED) { editor.update { enqueue(message) } }
      runCurrent()

      val firstUpdate = requireNotNull(first.await())
      val secondUpdate = requireNotNull(second.await())
      assertEquals(listOf(1L, 2L), fake.enqueuedRequests.map { it.id.value })
      assertEquals(1, fake.tickCount)
      assertEquals(firstUpdate.revision, secondUpdate.revision)
      assertEquals(listOf(CommandOutcome.Applied), firstUpdate.commandOutcomes)
      assertEquals(
        listOf(CommandOutcome.Rejected(CommandRejection.InvalidArgument)),
        secondUpdate.commandOutcomes,
      )
    }

  @Test
  fun updateNowTicksThroughTheExactRequest() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val update = requireNotNull(editor.updateNow { enqueue(message) })

      assertEquals(listOf(1L), fake.enqueuedRequests.map { it.id.value })
      assertEquals(fake.enqueuedRequests.single().id, fake.tickThroughRequests.single())
      assertEquals(1L, update.revision)
      assertEquals(listOf(CommandOutcome.Applied), update.commandOutcomes)
    }

  @Test
  fun malformedFrozenCommentDoesNotFailEditorOrBlockValidSibling() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val validSelection = assertNotNull(fake.freezeSelection(FakeFfiEditor.EmptySelection))

      editor.addFrozenComment(
        id = "malformed",
        group = COMMENT_RANGE_GROUP,
        selection = buildJsonObject { put("version", "invalid") },
      )
      editor.addFrozenComment(
        id = "valid",
        group = COMMENT_RANGE_GROUP,
        selection = json.encodeToJsonElement(validSelection),
      )

      assertFalse(editor.terminal)
      assertEquals(
        listOf("valid"),
        fake.enqueued.mapNotNull { message ->
          val op = (message as? Message.TrackedRange)?.op
          (op as? co.typie.editor.ffi.TrackedRangeOp.AddFrozen)?.id
        },
      )
    }

  @Test
  fun tickThroughLeavesLaterResourceWorkQueued() {
    val fake = FakeFfiEditor()
    val requestId = fake.enqueueRequest(listOf(message))
    fake.receiveResourceUpdate(FakeResourceUpdate())

    val through = fake.tickThrough(requestId)
    val remaining = assertNotNull(fake.tick())

    assertEquals(listOf(requestId), through.requestOutcomes.map { it.requestId })
    assertEquals(emptyList(), remaining.requestOutcomes)
    assertEquals(through.revision.value + 1L, remaining.revision.value)
  }

  @Test
  fun tickThroughConsumesResourceWorkBeforeTheExactRequest() {
    val fake = FakeFfiEditor()
    val resource = FakeResourceUpdate()
    fake.receiveResourceUpdate(resource)
    val requestId = fake.enqueueRequest(listOf(message))

    val through = fake.tickThrough(requestId)

    assertSame(resource, fake.receivedResourceUpdates.single())
    assertEquals(listOf(requestId), through.requestOutcomes.map { it.requestId })
    assertNull(fake.tick())
  }

  @Test
  fun awaitPublishedDistinguishesNoHostFromAnActiveHostWithNoTargets() =
    runTest(dispatcher) {
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      val withoutHost =
        async(start = CoroutineStart.UNDISPATCHED) { editor.update { enqueue(message) } }
      runCurrent()
      val noHostUpdate = requireNotNull(withoutHost.await())

      assertSame(NoHost, noHostUpdate.awaitPublished())

      val token = Any()
      editor.activateVisualHost(token)
      val withHost =
        async(start = CoroutineStart.UNDISPATCHED) { editor.update { enqueue(message) } }
      runCurrent()
      val publication = requireNotNull(withHost.await()).awaitPublished()

      val published = assertIs<Published>(publication)
      assertEquals(2L, published.revision)
    }
}
