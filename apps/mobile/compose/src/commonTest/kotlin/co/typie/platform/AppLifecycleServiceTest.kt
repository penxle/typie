package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class AppLifecycleServiceTest {
  @Test
  fun initialForegroundStateIsBaselineNotReturnEvent() {
    val service = AppLifecycleService()

    service.update(foreground = true)
    service.update(foreground = true)

    assertEquals(AppLifecycleState.Foreground, service.snapshot.value.state)
    assertEquals(0, service.snapshot.value.foregroundGeneration)
  }

  @Test
  fun firstForegroundAfterBackgroundBaselineIsNotReturnEvent() {
    val service = AppLifecycleService()

    service.update(foreground = false)
    service.update(foreground = true)

    assertEquals(AppLifecycleState.Foreground, service.snapshot.value.state)
    assertEquals(0, service.snapshot.value.foregroundGeneration)
  }

  @Test
  fun foregroundGenerationAdvancesOnlyAfterBackground() {
    val service = AppLifecycleService()
    service.update(foreground = true)

    service.update(foreground = false)
    service.update(foreground = true)
    service.update(foreground = true)

    assertEquals(AppLifecycleState.Foreground, service.snapshot.value.state)
    assertEquals(1, service.snapshot.value.foregroundGeneration)
  }

  @Test
  fun foregroundTransitionPublishesOneAtomicSnapshot() = runTest {
    val service = AppLifecycleService().apply { update(foreground = true) }
    val observed = mutableListOf<Pair<AppLifecycleState, Long>>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      service.snapshot.drop(1).collect { observed += it.state to it.foregroundGeneration }
    }

    service.update(foreground = false)
    service.update(foreground = true)

    assertEquals(
      listOf(AppLifecycleState.Background to 0L, AppLifecycleState.Foreground to 1L),
      observed,
    )
  }
}
