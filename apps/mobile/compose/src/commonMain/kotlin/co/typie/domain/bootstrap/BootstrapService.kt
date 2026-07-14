package co.typie.domain.bootstrap

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.typie.domain.auth.AuthService
import co.typie.domain.auth.AuthState
import co.typie.domain.preflight.PreflightService
import co.typie.domain.preflight.PreflightState
import co.typie.graphql.Apollo
import co.typie.graphql.BootstrapService_Query
import co.typie.migration.LegacyMigrationCoordinator
import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.first

object BootstrapService {
  var state by mutableStateOf<BootstrapState>(BootstrapState.NotReady)
    private set

  suspend fun launch() {
    PreflightService.launch()

    val preflight = snapshotFlow { PreflightService.state }.first { it !is PreflightState.NotReady }
    if (preflight is PreflightState.Ready || preflight is PreflightState.Unavailable) {
      try {
        LegacyMigrationCoordinator.runIfNeeded()
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        // best effort
      }

      if (AuthService.state !is AuthState.Authenticated) {
        try {
          AuthService.renew()
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          // best effort
        }
      }

      if (AuthService.state is AuthState.Authenticated) {
        try {
          ensureSiteId()
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          // best effort
        }
      }
    }

    try {
      PlatformModule.purchaseService.launch()
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      // best effort
    }

    state = BootstrapState.Ready
  }

  private suspend fun ensureSiteId() {
    val response = Apollo.query(BootstrapService_Query()).execute()
    val siteIds = response.dataOrThrow().me.sites.map { it.id }

    val siteId = Preference.siteId
    if (siteId != null && siteId in siteIds) {
      return
    }

    Preference.siteId = siteIds.first()
  }
}
