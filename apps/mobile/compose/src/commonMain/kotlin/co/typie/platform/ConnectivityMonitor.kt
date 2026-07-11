package co.typie.platform

import kotlinx.coroutines.flow.Flow

expect fun connectivityAvailabilityFlow(): Flow<Boolean>
