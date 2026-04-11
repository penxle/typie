package co.typie.bootstrap

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.typie.auth.AuthService
import co.typie.auth.AuthState
import co.typie.graphql.Apollo
import co.typie.graphql.BootstrapService_Query
import co.typie.preflight.PreflightService
import co.typie.preflight.PreflightState
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
        AuthService.renew()
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        // best effort
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
