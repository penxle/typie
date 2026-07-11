package co.typie.platform

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

internal class ConnectivityService(private val availability: Flow<Boolean>) {
  private val mutableRestorationGeneration = MutableStateFlow(0L)

  val restorationGeneration: StateFlow<Long> = mutableRestorationGeneration.asStateFlow()

  suspend fun monitor() {
    var previous: Boolean? = null
    availability.collect { available ->
      if (previous == false && available) {
        mutableRestorationGeneration.value += 1
      }
      previous = available
    }
  }
}

internal val connectivityService = ConnectivityService(connectivityAvailabilityFlow())
