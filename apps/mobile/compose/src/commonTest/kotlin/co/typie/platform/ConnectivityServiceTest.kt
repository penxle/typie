package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class ConnectivityServiceTest {
  @Test
  fun firstAvailabilityIsBaseline() = runTest {
    val availability = MutableSharedFlow<Boolean>()
    val service = ConnectivityService(availability)
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { service.monitor() }

    availability.emit(true)

    assertEquals(0, service.restorationGeneration.value)
  }

  @Test
  fun generationAdvancesOnlyFromUnavailableToAvailable() = runTest {
    val availability = MutableSharedFlow<Boolean>()
    val service = ConnectivityService(availability)
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { service.monitor() }

    availability.emit(true)
    availability.emit(true)
    availability.emit(false)
    availability.emit(false)
    availability.emit(true)
    availability.emit(true)

    assertEquals(1, service.restorationGeneration.value)
  }
}
