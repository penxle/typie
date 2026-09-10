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
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.cancel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestCoroutineScheduler
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorRegistryTest {
  private val registry = EditorRegistry()

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
    registry.register(editor)
    try {
      assertTrue(registry.snapshot().contains(editor))
    } finally {
      registry.unregister(editor)
    }
  }

  @Test
  fun unregistered_editor_leaves_snapshot() = runTest {
    val editor = makeEditor()
    registry.register(editor)
    registry.unregister(editor)

    assertFalse(registry.snapshot().contains(editor))
  }

  @Test
  fun concurrent_register_unregister_is_consistent() = runTest {
    val editors = List(20) { makeEditor() }
    val jobs = editors.map { e ->
      async(Dispatchers.Default) {
        registry.register(e)
        registry.unregister(e)
      }
    }
    jobs.awaitAll()

    val snap = registry.snapshot()
    for (e in editors) {
      assertFalse(snap.contains(e), "editor $e unexpectedly in snapshot")
    }
  }

  @Test
  fun no_op_resource_commit_does_not_fan_out_or_schedule_tick() = runTest {
    val fake = FakeFfiEditor()
    val editor = makeEditor(this, fake, StandardTestDispatcher(testScheduler))
    registry.register(editor)
    try {
      advanceUntilIdle()
      fake.receivedResourceUpdates.clear()
      fake.tickCount = 0

      registry.commitResourceUpdate { null }
      advanceUntilIdle()

      assertTrue(fake.receivedResourceUpdates.isEmpty())
      assertEquals(0, fake.tickCount)
    } finally {
      registry.unregister(editor)
    }
  }

  @Test
  fun resource_commit_fans_out_exact_update_without_waiting_for_tick() = runTest {
    val firstFake = FakeFfiEditor()
    val secondFake = FakeFfiEditor()
    // Keep editor ticks paused while the commit coroutine returns from Dispatchers.Default.
    val editorScheduler = TestCoroutineScheduler()
    val dispatcher = StandardTestDispatcher(editorScheduler)
    val editorScope = CoroutineScope(SupervisorJob() + dispatcher)
    val first = makeEditor(editorScope, firstFake, dispatcher)
    val second = makeEditor(editorScope, secondFake, dispatcher)
    val update = FakeResourceUpdate()
    registry.register(first)
    registry.register(second)
    try {
      editorScheduler.advanceUntilIdle()
      firstFake.receivedResourceUpdates.clear()
      secondFake.receivedResourceUpdates.clear()
      firstFake.tickCount = 0
      secondFake.tickCount = 0

      registry.commitResourceUpdate { update }

      assertSame(update, firstFake.receivedResourceUpdates.last())
      assertSame(update, secondFake.receivedResourceUpdates.last())
      assertEquals(0, firstFake.tickCount)
      assertEquals(0, secondFake.tickCount)

      editorScheduler.advanceUntilIdle()
      assertEquals(1, firstFake.tickCount)
      assertEquals(1, secondFake.tickCount)
    } finally {
      registry.unregister(first)
      registry.unregister(second)
      editorScope.cancel()
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
    registry.register(first)
    registry.register(second)
    try {
      advanceUntilIdle()
      firstFake.receivedResourceUpdates.clear()
      secondFake.receivedResourceUpdates.clear()

      val job = launch {
        val self = currentCoroutineContext()[Job]!!
        registry.commitResourceUpdate {
          self.cancel()
          update
        }
      }
      runCurrent()
      job.join()

      assertSame(update, firstFake.receivedResourceUpdates.last())
      assertSame(update, secondFake.receivedResourceUpdates.last())
    } finally {
      registry.unregister(first)
      registry.unregister(second)
    }
  }

  @Test
  fun registration_catches_up_with_the_latest_committed_update() = runTest {
    val update = FakeResourceUpdate()
    registry.commitResourceUpdate { update }

    val fake = FakeFfiEditor()
    val editor = makeEditor(this, fake, StandardTestDispatcher(testScheduler))
    try {
      registry.register(editor)

      assertSame(update, fake.receivedResourceUpdates.last())
      assertEquals(0, fake.tickCount)
    } finally {
      registry.unregister(editor)
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
          async(Dispatchers.Default) { registry.register(editor) },
          async(Dispatchers.Default) { registry.commitResourceUpdate { update } },
        )

        assertTrue(fake.receivedResourceUpdates.contains(update))
      } finally {
        registry.unregister(editor)
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
    registry.register(failed)
    registry.register(healthy)
    try {
      failedFake.receiveResourceUpdateProvider = { error("resource update admission failed") }
      registry.commitResourceUpdate { update }

      assertFalse(registry.snapshot().contains(failed))
      assertTrue(registry.snapshot().contains(healthy))
      assertSame(update, healthyFake.receivedResourceUpdates.last())
    } finally {
      registry.unregister(failed)
      registry.unregister(healthy)
    }
  }
}
