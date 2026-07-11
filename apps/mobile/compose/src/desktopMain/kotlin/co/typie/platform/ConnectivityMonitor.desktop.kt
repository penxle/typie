package co.typie.platform

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

actual fun connectivityAvailabilityFlow(): Flow<Boolean> = emptyFlow()
