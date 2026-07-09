package co.typie.platform

import kotlinx.coroutines.flow.Flow

expect fun connectivityRestoredFlow(): Flow<Unit>
