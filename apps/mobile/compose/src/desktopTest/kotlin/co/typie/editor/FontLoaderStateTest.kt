package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class FontLoaderStateTest {
  @Test
  fun same_hash_rollback_is_stale_via_generation() {
    val s = FontLoaderState()
    val fk = "Pretendard:400"
    val dispatched = s.generationOf(fk) // A 세대에서 파견
    s.purge(listOf(fk), mutableSetOf()) // A→B
    s.purge(listOf(fk), mutableSetOf()) // B→A (동일 hash 롤백)
    assertTrue(s.isStale(fk, dispatched))
  }

  @Test
  fun purge_clears_all_kind_keys_and_queued() {
    val s = FontLoaderState()
    val fk = "Pretendard:400"
    s.loaded.add("manifest:$fk:h1")
    s.loaded.add("base:$fk:h1")
    s.retryScheduled["chunk:$fk:h1:3"] = RetryChain(0L, 2)
    val queued = mutableSetOf("manifest:$fk:h1")
    s.purge(listOf(fk), queued)
    assertTrue(s.loaded.isEmpty())
    assertTrue(s.retryScheduled.isEmpty())
    assertTrue(queued.isEmpty())
  }

  @Test
  fun unrelated_font_keys_survive_purge() {
    val s = FontLoaderState()
    s.loaded.add("base:Other:700:h9")
    s.purge(listOf("Pretendard:400"), mutableSetOf())
    assertTrue("base:Other:700:h9" in s.loaded)
  }

  @Test
  fun stale_owner_cannot_remove_new_entry_aba() {
    val s = FontLoaderState()
    val key = "manifest:Pretendard:400:h1"
    val old = CompletableDeferred<Unit>()
    s.loading[key] = old
    s.purge(listOf("Pretendard:400"), mutableSetOf()) // A→B: 구 엔트리 제거
    val fresh = CompletableDeferred<Unit>()
    s.loading[key] = fresh // B→A 롤백 후 신 작업 등록
    // 구 작업의 정리 규약: 자기 인스턴스일 때만 제거
    if (s.loading[key] === old) s.loading.remove(key)
    assertTrue(s.loading[key] === fresh, "구 작업이 신 엔트리를 지우면 안 된다")
  }

  @Test
  fun injection_and_commit_are_serialized_with_concurrent_purge() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val key = "manifest:Pretendard:400:h1"
    val events = mutableListOf<String>()

    val loadJob = launch {
      state.loadOnce(mutex, key) {
        mutex.withLock {
          events.add("inject-start")
          delay(50)
          state.loaded.add(key)
          events.add("inject-end")
        }
        true
      }
    }
    val purgeJob = launch {
      mutex.withLock {
        events.add("purge-start")
        events.add("purge-end")
      }
    }

    advanceUntilIdle()
    loadJob.join()
    purgeJob.join()

    assertEquals(listOf("inject-start", "inject-end", "purge-start", "purge-end"), events)
  }

  @Test
  fun waiter_receives_false_when_leader_stale_discards() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val key = "manifest:Pretendard:400:h1"
    val release = CompletableDeferred<Unit>()

    var leaderResult: Boolean? = null
    var waiterResult: Boolean? = null
    var waiterBlockCalls = 0

    val leaderJob = launch {
      leaderResult =
        state.loadOnce(mutex, key) {
          release.await()
          false
        }
    }
    runCurrent()

    val waiterJob = launch {
      waiterResult =
        state.loadOnce(mutex, key) {
          waiterBlockCalls++
          true
        }
    }
    runCurrent()

    release.complete(Unit)
    advanceUntilIdle()
    leaderJob.join()
    waiterJob.join()

    assertEquals(false, leaderResult)
    assertEquals(false, waiterResult)
    assertEquals(0, waiterBlockCalls)
    assertFalse(key in state.loaded)
  }

  @Test
  fun retry_joins_inflight_and_ends_chain_on_success() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val scope = CoroutineScope(coroutineContext)
    val key = "manifest:Pretendard:400:h1"

    val inflight = CompletableDeferred<Unit>()
    mutex.withLock { state.loading[key] = inflight }

    var blockCalls = 0
    state.scheduleRetry(mutex, scope, key, 0L) { blockCalls++ }

    advanceTimeBy(RETRY_BASE_MS)
    runCurrent()

    mutex.withLock { state.loaded.add(key) }
    inflight.complete(Unit)
    runCurrent()

    assertEquals(0, blockCalls)
    assertNull(mutex.withLock { state.retryScheduled[key] })
  }

  @Test
  fun retry_chain_increments_attempt_then_self_destructs_at_cap() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val scope = CoroutineScope(coroutineContext)
    val key = "manifest:Pretendard:400:h1"

    val inflight = CompletableDeferred<Unit>()
    mutex.withLock { state.loading[key] = inflight }
    inflight.complete(
      Unit
    ) // settled without ever becoming loaded: every join resolves instantly as a failure

    var blockCalls = 0
    state.scheduleRetry(mutex, scope, key, 0L) { blockCalls++ }
    assertEquals(RetryChain(0L, 1), mutex.withLock { state.retryScheduled[key] })

    advanceTimeBy(RETRY_BASE_MS)
    runCurrent()
    assertEquals(RetryChain(0L, 1), mutex.withLock { state.retryScheduled[key] })

    advanceTimeBy(RETRY_BASE_MS * 2)
    runCurrent()
    assertEquals(RetryChain(0L, 2), mutex.withLock { state.retryScheduled[key] })

    advanceTimeBy(RETRY_BASE_MS * 4)
    runCurrent()
    assertEquals(RetryChain(0L, 3), mutex.withLock { state.retryScheduled[key] })

    advanceTimeBy(RETRY_BASE_MS * 8)
    runCurrent()
    assertEquals(RetryChain(0L, 4), mutex.withLock { state.retryScheduled[key] })

    advanceTimeBy(RETRY_CAP_MS)
    runCurrent()

    assertEquals(0, blockCalls)
    assertNull(mutex.withLock { state.retryScheduled[key] })
  }

  @Test
  fun retry_step_self_destructs_when_ownership_lost() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val scope = CoroutineScope(coroutineContext)
    val key = "manifest:Pretendard:400:h1"

    var blockCalls = 0
    state.scheduleRetry(mutex, scope, key, 0L) { blockCalls++ }

    mutex.withLock { state.retryScheduled[key] = RetryChain(1L, 1) }

    advanceTimeBy(RETRY_BASE_MS)
    runCurrent()

    assertEquals(0, blockCalls)
    assertEquals(RetryChain(1L, 1), mutex.withLock { state.retryScheduled[key] })
  }

  @Test
  fun preload_ownership_finally_does_not_clear_new_entry() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val scope = CoroutineScope(coroutineContext)
    val queue = FontLoader.PreloadQueue(mutex, state, scope)

    val releaseA = CompletableDeferred<Unit>()
    val releaseB = CompletableDeferred<Unit>()
    var bRuns = 0
    var cRuns = 0

    queue.enqueue("k", 0) { releaseA.await() }
    runCurrent()

    mutex.withLock { queue.purge(listOf("k")) }

    queue.enqueue("k", 0) {
      bRuns++
      releaseB.await()
    }
    runCurrent()

    releaseA.complete(Unit)
    runCurrent()

    queue.enqueue("k", 0) { cRuns++ }
    runCurrent()

    releaseB.complete(Unit)
    runCurrent()

    assertEquals(1, bRuns)
    assertEquals(0, cRuns)
  }

  @Test
  fun purge_after_commit_leaves_no_stale_loaded_key_and_allows_regen_fetch() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val fk = "Pretendard:400"
    val keyA = "manifest:$fk:hashA"

    val genA = state.generationOf(fk)
    val committedA =
      state.loadOnce(mutex, keyA) {
        if (state.isStale(fk, genA)) false
        else {
          state.loaded.add(keyA)
          true
        }
      }
    assertTrue(committedA)
    assertTrue(keyA in state.loaded)

    mutex.withLock { state.purge(listOf(fk), mutableSetOf()) }
    assertTrue(state.isStale(fk, genA))
    assertFalse(keyA in state.loaded)

    // late completion for the stale A dispatch must not resurrect the purged key
    val staleRedispatchCommitted =
      if (state.isStale(fk, genA)) {
        false
      } else {
        state.loadOnce(mutex, keyA) { true }
      }
    assertFalse(staleRedispatchCommitted)
    assertFalse(keyA in state.loaded)

    val genB = state.generationOf(fk)
    val keyB = "manifest:$fk:hashB"
    val committedB =
      if (state.isStale(fk, genB)) {
        false
      } else {
        state.loadOnce(mutex, keyB) {
          state.loaded.add(keyB)
          true
        }
      }
    assertTrue(committedB)
    assertTrue(keyB in state.loaded)
  }

  @Test
  fun preload_drains_in_ascending_priority_order() = runTest {
    val mutex = Mutex()
    val state = FontLoaderState()
    val scope = CoroutineScope(coroutineContext)
    val queue = FontLoader.PreloadQueue(mutex, state, scope)

    val started = mutableListOf<String>()
    val blockers = mutableListOf<CompletableDeferred<Unit>>()
    suspend fun blocked(key: String) {
      started.add(key)
      val gate = CompletableDeferred<Unit>()
      blockers.add(gate)
      gate.await()
    }

    repeat(4) { i -> queue.enqueue("order:fill:$i", 100) { blocked("order:fill:$i") } }
    queue.enqueue("order:chunk:9", 9) { blocked("order:chunk:9") }
    queue.enqueue("order:chunk:2", 2) { blocked("order:chunk:2") }
    queue.enqueue("order:manifest", -2) { blocked("order:manifest") }
    queue.enqueue("order:base", -1) { blocked("order:base") }
    runCurrent()

    blockers[0].complete(Unit)
    runCurrent()
    assertEquals("order:manifest", started[4])
    blockers[4].complete(Unit)
    runCurrent()
    assertEquals("order:base", started[5])
    blockers[5].complete(Unit)
    runCurrent()
    assertEquals("order:chunk:2", started[6])
    blockers[6].complete(Unit)
    runCurrent()
    assertEquals("order:chunk:9", started[7])

    blockers.forEach { it.complete(Unit) }
    runCurrent()
  }
}
