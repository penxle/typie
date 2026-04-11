package co.typie.bootstrap

import co.typie.auth.AuthService
import co.typie.auth.AuthState
import co.typie.graphql.Apollo
import co.typie.graphql.BootstrapService_Query
import co.typie.preflight.PreflightService
import co.typie.preflight.PreflightState
import co.typie.storage.Preference
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first

object BootstrapService {
  private val _state = MutableStateFlow<BootstrapState>(BootstrapState.NotReady)
  val state: StateFlow<BootstrapState> = _state

  suspend fun launch() {
    PreflightService.launch()

    val preflight = PreflightService.state.first { it !is PreflightState.NotReady }
    if (preflight is PreflightState.Ready || preflight is PreflightState.Unavailable) {
      try {
        AuthService.renew()
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        // best effort
      }

      if (AuthService.state.value is AuthState.Authenticated) {
        try {
          ensureSiteId()
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          // best effort
        }
      }
    }

    _state.value = BootstrapState.Ready
  }

  private suspend fun ensureSiteId() {
    val response = Apollo.query(BootstrapService_Query()).execute()
    val siteIds = response.dataOrThrow().me.sites.map { it.id }

    val siteId = Preference.siteId.value
    if (siteId != null && siteId in siteIds) {
      return
    }

    Preference.siteId.value = siteIds.first()
  }
}
