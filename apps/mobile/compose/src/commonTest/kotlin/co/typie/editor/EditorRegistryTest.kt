package co.typie.editor

import co.typie.editor.ffi.ResourceUpdate
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorRegistryTest {
  private class FakeResourceUpdate : ResourceUpdate

  private fun makeEditor(): Editor = Editor(FakeFfiEditor(), CoroutineScope(Dispatchers.Unconfined))

  private fun makeEditor(
    scope: CoroutineScope,
    fake: FakeFfiEditor,
    dispatcher: CoroutineDispatcher,
  ): Editor = Editor(fake, scope, dispatcher)

  @Test
  fun registered_editor_appears_in_snapshot() = runTest {
    val editor = makeEditor()
    EditorRegistry.register(editor)
    try {
      assertTrue(EditorRegistry.snapshot().contains(editor))
    } finally {
      EditorRegistry.unregister(editor)
    }
  }

  @Test
  fun unregistered_editor_leaves_snapshot() = runTest {
    val editor = makeEditor()
    EditorRegistry.register(editor)
    EditorRegistry.unregister(editor)

    assertFalse(EditorRegistry.snapshot().contains(editor))
  }

  @Test
  fun concurrent_register_unregister_is_consistent() = runTest {
    val editors = List(20) { makeEditor() }
    val jobs = editors.map { e ->
      async(Dispatchers.Default) {
        EditorRegistry.register(e)
        EditorRegistry.unregister(e)
      }
    }
    jobs.awaitAll()

    val snap = EditorRegistry.snapshot()
    for (e in editors) {
      assertFalse(snap.contains(e), "editor $e unexpectedly in snapshot")
    }
  }

  @Test
  fun no_op_resource_commit_does_not_fan_out_or_schedule_tick() = runTest {
    val fake = FakeFfiEditor()
    val editor = makeEditor(this, fake, StandardTestDispatcher(testScheduler))
    EditorRegistry.register(editor)
    try {
      advanceUntilIdle()
      fake.receivedResourceUpdates.clear()
      fake.tickCount = 0

      EditorRegistry.commitResourceUpdate { null }
      advanceUntilIdle()

      assertTrue(fake.receivedResourceUpdates.isEmpty())
      assertEquals(0, fake.tickCount)
    } finally {
      EditorRegistry.unregister(editor)
    }
  }

  @Test
  fun resource_commit_fans_out_exact_update_without_waiting_for_tick() = runTest {
    val firstFake = FakeFfiEditor()
    val secondFake = FakeFfiEditor()
    val dispatcher = StandardTestDispatcher(testScheduler)
    val first = makeEditor(this, firstFake, dispatcher)
    val second = makeEditor(this, secondFake, dispatcher)
    val update = FakeResourceUpdate()
    EditorRegistry.register(first)
    EditorRegistry.register(second)
    try {
      advanceUntilIdle()
      firstFake.receivedResourceUpdates.clear()
      secondFake.receivedResourceUpdates.clear()
      firstFake.tickCount = 0
      secondFake.tickCount = 0

      EditorRegistry.commitResourceUpdate { update }

      assertSame(update, firstFake.receivedResourceUpdates.last())
      assertSame(update, secondFake.receivedResourceUpdates.last())
      assertEquals(0, firstFake.tickCount)
      assertEquals(0, secondFake.tickCount)

      advanceUntilIdle()
      assertEquals(1, firstFake.tickCount)
      assertEquals(1, secondFake.tickCount)
    } finally {
      EditorRegistry.unregister(first)
      EditorRegistry.unregister(second)
    }
  }

  @Test
  fun cancellation_after_commit_still_fans_out_to_every_registered_editor() = runTest {
    val firstFake = FakeFfiEditor()
    val secondFake = FakeFfiEditor()
    val dispatcher = StandardTestDispatcher(testScheduler)
    val first = makeEditor(this, firstFake, dispatcher)
    val second = makeEditor(this, secondFake, dispatcher)
    val update = FakeResourceUpdate()
    EditorRegistry.register(first)
    EditorRegistry.register(second)
    try {
      advanceUntilIdle()
      firstFake.receivedResourceUpdates.clear()
      secondFake.receivedResourceUpdates.clear()

      val job = launch {
        val self = currentCoroutineContext()[Job]!!
        EditorRegistry.commitResourceUpdate {
          self.cancel()
          update
        }
      }
      runCurrent()
      job.join()

      assertSame(update, firstFake.receivedResourceUpdates.last())
      assertSame(update, secondFake.receivedResourceUpdates.last())
    } finally {
      EditorRegistry.unregister(first)
      EditorRegistry.unregister(second)
    }
  }

  @Test
  fun registration_catches_up_with_the_latest_committed_update() = runTest {
    val update = FakeResourceUpdate()
    EditorRegistry.commitResourceUpdate { update }

    val fake = FakeFfiEditor()
    val editor = makeEditor(this, fake, StandardTestDispatcher(testScheduler))
    try {
      EditorRegistry.register(editor)

      assertSame(update, fake.receivedResourceUpdates.last())
      assertEquals(0, fake.tickCount)
    } finally {
      EditorRegistry.unregister(editor)
    }
  }

  @Test
  fun concurrent_registration_and_commit_never_miss_the_update() = runTest {
    repeat(20) {
      val update = FakeResourceUpdate()
      val fake = FakeFfiEditor()
      val editor = makeEditor(this, fake, StandardTestDispatcher(testScheduler))
      try {
        awaitAll(
          async(Dispatchers.Default) { EditorRegistry.register(editor) },
          async(Dispatchers.Default) { EditorRegistry.commitResourceUpdate { update } },
        )

        assertTrue(fake.receivedResourceUpdates.contains(update))
      } finally {
        EditorRegistry.unregister(editor)
      }
    }
  }

  @Test
  fun admission_failure_removes_only_the_failed_editor() = runTest {
    val failedFake = FakeFfiEditor()
    val healthyFake = FakeFfiEditor()
    val dispatcher = StandardTestDispatcher(testScheduler)
    val failed = makeEditor(this, failedFake, dispatcher)
    val healthy = makeEditor(this, healthyFake, dispatcher)
    val update = FakeResourceUpdate()
    EditorRegistry.register(failed)
    EditorRegistry.register(healthy)
    try {
      failedFake.receiveResourceUpdateProvider = { error("resource update admission failed") }
      EditorRegistry.commitResourceUpdate { update }

      assertFalse(EditorRegistry.snapshot().contains(failed))
      assertTrue(EditorRegistry.snapshot().contains(healthy))
      assertSame(update, healthyFake.receivedResourceUpdates.last())
    } finally {
      EditorRegistry.unregister(failed)
      EditorRegistry.unregister(healthy)
    }
  }
}
