package co.typie.screen.editor.editor

import co.typie.editor.external.EditorAssetResolution
import co.typie.editor.external.EditorExternalAsset
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorAssetHydratorTest {
  @Test
  fun seededAssetsDoNotFetch() = runTest {
    val state = EditorExternalElementState()
    val calls = mutableListOf<List<String>>()
    val hydrator =
      hydrator(state) { ids ->
        calls += ids
        emptyList()
      }
    val image = imageAsset("IMG0SEEDED")

    hydrator.seed(listOf(image))
    hydrator.resolve(listOf(image.id))

    assertEquals(emptyList(), calls)
    assertEquals(image, state.images.assets[image.id])
  }

  @Test
  fun duplicateMixedReferencesBecomeOneCanonicalMissingBatch() = runTest {
    val state = EditorExternalElementState()
    val calls = mutableListOf<List<String>>()
    val seeded = imageAsset("IMG0SEEDED")
    val file = fileAsset("FILE0MISSING")
    val image = imageAsset("IMG0MISSING")
    val assets = mapOf(file.id to file, image.id to image)
    val hydrator =
      hydrator(state) { ids ->
        calls += ids
        ids.mapNotNull(assets::get)
      }

    hydrator.seed(listOf(seeded))
    hydrator.resolve(listOf(file.id, seeded.id, image.id, file.id))

    assertEquals(listOf(listOf(file.id, image.id)), calls)
    assertEquals(file, state.files.assets[file.id])
    assertEquals(image, state.images.assets[image.id])
  }

  @Test
  fun missingReferencesAreSplitIntoBoundedCanonicalBatches() = runTest {
    val state = EditorExternalElementState()
    val calls = mutableListOf<List<String>>()
    val ids = (0..50).map { index -> "IMG${index.toString().padStart(8, '0')}" }
    val assets = ids.associateWith(::imageAsset)
    val hydrator =
      hydrator(state) { batch ->
        calls += batch
        batch.mapNotNull(assets::get)
      }

    hydrator.resolve(ids.reversed())

    assertEquals(listOf(50, 1), calls.map(List<String>::size))
    assertEquals(ids, calls.flatten())
  }

  @Test
  fun overlappingReferenceUpdatesDoNotDuplicateRequests() = runTest {
    val state = EditorExternalElementState()
    val calls = mutableListOf<List<String>>()
    val releaseFirst = CompletableDeferred<Unit>()
    val first = imageAsset("IMG0FIRST")
    val second = fileAsset("FILE0SECOND")
    val assets = mapOf(first.id to first, second.id to second)
    val hydrator =
      hydrator(state) { ids ->
        calls += ids
        if (calls.size == 1) releaseFirst.await()
        ids.mapNotNull(assets::get)
      }

    val firstJob = launch { hydrator.resolve(listOf(first.id)) }
    runCurrent()
    val secondJob = launch { hydrator.resolve(listOf(first.id, second.id)) }
    runCurrent()

    assertEquals(listOf(listOf(first.id)), calls)
    releaseFirst.complete(Unit)
    advanceUntilIdle()

    assertEquals(listOf(listOf(first.id), listOf(second.id)), calls)
    assertTrue(firstJob.isCompleted)
    assertTrue(secondJob.isCompleted)
  }

  @Test
  fun referenceJoiningDuringBackoffReachesUnavailableWithItsOwnAttemptBudget() = runTest {
    val state = EditorExternalElementState()
    val firstId = "IMG0FIRST"
    val joinedId = "FILE0JOINED"
    val firstBackoffStarted = CompletableDeferred<Unit>()
    val releaseFirstBackoff = CompletableDeferred<Unit>()
    var backoffs = 0
    val hydrator =
      EditorAssetHydrator(
        state = state,
        fetch = { emptyList() },
        waitBeforeRetry = {
          backoffs += 1
          if (backoffs == 1) {
            firstBackoffStarted.complete(Unit)
            releaseFirstBackoff.await()
          }
        },
      )

    val firstJob = launch { hydrator.resolve(listOf(firstId)) }
    runCurrent()
    assertTrue(firstBackoffStarted.isCompleted)

    val joinedJob = launch { hydrator.resolve(listOf(firstId, joinedId)) }
    runCurrent()
    releaseFirstBackoff.complete(Unit)
    advanceUntilIdle()

    assertTrue(firstJob.isCompleted)
    assertTrue(joinedJob.isCompleted)
    assertEquals(EditorAssetResolution.Unavailable, state.resolutions[firstId])
    assertEquals(EditorAssetResolution.Unavailable, state.resolutions[joinedId])
  }

  @Test
  fun successfulOmissionRetriesExactlyThreeTimesThenBecomesUnavailable() = runTest {
    val state = EditorExternalElementState()
    var calls = 0
    val delayedAttempts = mutableListOf<Int>()
    val id = "IMG0LATE"
    val hydrator =
      EditorAssetHydrator(
        state = state,
        fetch = {
          calls += 1
          emptyList()
        },
        waitBeforeRetry = { attempt ->
          delayedAttempts += attempt
          delay(1)
        },
      )

    hydrator.resolve(listOf(id))

    assertEquals(3, calls)
    assertEquals(listOf(1, 2), delayedAttempts)
    assertEquals(EditorAssetResolution.Unavailable, state.resolutions[id])
  }

  @Test
  fun queryFailureIsSuppressedUntilConnectivityRecovery() = runTest {
    val state = EditorExternalElementState()
    val id = "IMG0RETRY"
    var calls = 0
    var fail = true
    val asset = imageAsset(id)
    val hydrator =
      hydrator(state) {
        calls += 1
        if (fail) error("offline")
        listOf(asset)
      }

    hydrator.resolve(listOf(id))
    hydrator.resolve(listOf(id))

    assertEquals(1, calls)
    assertEquals(EditorAssetResolution.RetryableFailure, state.resolutions[id])

    fail = false
    hydrator.onConnectivityRestored(generation = 1)
    hydrator.onConnectivityRestored(generation = 1)

    assertEquals(2, calls)
    assertEquals(asset, state.images.assets[id])
    assertNull(state.resolutions[id])
  }

  @Test
  fun connectivityRecoveryDuringInFlightFailureRetriesInNewGeneration() = runTest {
    val state = EditorExternalElementState()
    val id = "IMG0RECOVERED"
    val asset = imageAsset(id)
    val firstFetchStarted = CompletableDeferred<Unit>()
    val releaseFirstFetch = CompletableDeferred<Unit>()
    var calls = 0
    val hydrator =
      hydrator(state) {
        calls += 1
        if (calls == 1) {
          firstFetchStarted.complete(Unit)
          releaseFirstFetch.await()
          error("stale request failed")
        }
        listOf(asset)
      }

    val resolveJob = launch { hydrator.resolve(listOf(id)) }
    runCurrent()
    assertTrue(firstFetchStarted.isCompleted)

    val recoveryJob = launch { hydrator.onConnectivityRestored(generation = 1) }
    runCurrent()
    releaseFirstFetch.complete(Unit)
    advanceUntilIdle()

    assertTrue(resolveJob.isCompleted)
    assertTrue(recoveryJob.isCompleted)
    assertEquals(2, calls)
    assertEquals(asset, state.images.assets[id])
    assertNull(state.resolutions[id])
  }

  @Test
  fun connectivityGenerationGivesUnavailableOnlyOneProbeWithoutNewBackoffChain() = runTest {
    val state = EditorExternalElementState()
    val id = "IMG0UNAVAILABLE"
    var calls = 0
    val delayedAttempts = mutableListOf<Int>()
    val hydrator =
      EditorAssetHydrator(
        state = state,
        fetch = {
          calls += 1
          emptyList()
        },
        waitBeforeRetry = { attempt -> delayedAttempts += attempt },
      )

    hydrator.resolve(listOf(id))
    hydrator.onConnectivityRestored(generation = 1)
    hydrator.onConnectivityRestored(generation = 1)
    hydrator.onConnectivityRestored(generation = 2)

    assertEquals(5, calls)
    assertEquals(listOf(1, 2), delayedAttempts)
    assertEquals(EditorAssetResolution.Unavailable, state.resolutions[id])
  }

  @Test
  fun queryRefreshGenerationRestoresThreeAttemptBudgetExactlyOnce() = runTest {
    val state = EditorExternalElementState()
    val id = "FILE0REFRESH"
    var calls = 0
    val hydrator =
      hydrator(state) {
        calls += 1
        emptyList()
      }

    hydrator.resolve(listOf(id))
    assertEquals(3, calls)

    hydrator.onQueryRefresh(generation = 1, assets = emptyList())
    assertEquals(6, calls)
    hydrator.onQueryRefresh(generation = 1, assets = emptyList())
    assertEquals(6, calls)

    hydrator.onQueryRefresh(generation = 2, assets = emptyList())
    assertEquals(9, calls)
  }

  @Test
  fun removedReferencesClearTransientStateAndLateResponsesOnlyPopulateCache() = runTest {
    val state = EditorExternalElementState()
    val asset = imageAsset("IMG0REMOVED")
    val release = CompletableDeferred<Unit>()
    val hydrator =
      hydrator(state) {
        release.await()
        listOf(asset)
      }

    val resolving = launch { hydrator.resolve(listOf(asset.id)) }
    runCurrent()
    assertEquals(EditorAssetResolution.InFlight, state.resolutions[asset.id])

    val removing = launch { hydrator.resolve(emptyList()) }
    runCurrent()
    assertNull(state.resolutions[asset.id])

    release.complete(Unit)
    advanceUntilIdle()

    assertTrue(resolving.isCompleted)
    assertTrue(removing.isCompleted)
    assertEquals(asset, state.images.assets[asset.id])
    assertNull(state.resolutions[asset.id])
  }

  @Test
  fun cancellationClearsStaleInFlightState() = runTest {
    val state = EditorExternalElementState()
    val id = "IMG0CANCELLED"
    val hydrator = hydrator(state) { awaitCancellation() }

    val job = launch { hydrator.resolve(listOf(id)) }
    runCurrent()
    assertEquals(EditorAssetResolution.InFlight, state.resolutions[id])

    job.cancel()
    advanceUntilIdle()

    assertFalse(job.isActive)
    assertNull(state.resolutions[id])
  }

  private fun hydrator(
    state: EditorExternalElementState,
    fetch: suspend (List<String>) -> List<EditorExternalAsset>,
  ): EditorAssetHydrator = EditorAssetHydrator(state = state, fetch = fetch, waitBeforeRetry = {})

  private fun imageAsset(id: String): EditorImageAsset =
    EditorImageAsset(
      id = id,
      url = "https://example.com/$id",
      width = 100,
      height = 50,
      ratio = 2.0,
      placeholder = null,
    )

  private fun fileAsset(id: String): EditorFileAsset =
    EditorFileAsset(id = id, name = "$id.txt", url = "https://example.com/$id", size = 10)
}
