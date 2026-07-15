package co.typie.domain.pushnotification

import androidx.compose.runtime.snapshotFlow
import co.touchlab.kermit.Logger
import co.typie.domain.auth.AuthService
import co.typie.domain.auth.AuthState
import co.typie.domain.bootstrap.BootstrapService
import co.typie.domain.bootstrap.BootstrapState
import co.typie.graphql.Apollo
import co.typie.graphql.PushNotificationService_RegisterPushNotificationToken_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.RegisterPushNotificationTokenInput
import co.typie.network.isRecoverableNetworkError
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

object PushNotificationService {
  private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

  suspend fun launch() {
    snapshotFlow { BootstrapService.state }.first { it is BootstrapState.Ready }

    scope.launch {
      FirebaseMessaging.onTokenRefresh.collect { token ->
        if (AuthService.state is AuthState.Authenticated) {
          registerToken(token)
        }
      }
    }

    snapshotFlow { AuthService.state }
      .collect { state ->
        when (state) {
          is AuthState.Authenticated -> registerCurrentToken()
          AuthState.Unauthenticated -> safelyDeleteToken()
        }
      }
  }

  private suspend fun registerCurrentToken() {
    try {
      val granted = FirebaseMessaging.requestPermission()
      if (!granted) return
      val token = FirebaseMessaging.token() ?: return
      registerToken(token)
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.w(e) { "PushNotificationService: token fetch failed" }
      if (!e.isRecoverableNetworkError()) {
        Sentry.captureException(e)
      }
    }
  }

  private suspend fun registerToken(token: String) {
    try {
      Apollo.executeMutation(
        PushNotificationService_RegisterPushNotificationToken_Mutation(
          input = RegisterPushNotificationTokenInput(token = token)
        )
      )
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.w(e) { "PushNotificationService: token registration failed" }
      if (!e.isRecoverableNetworkError()) {
        Sentry.captureException(e)
      }
    }
  }

  private suspend fun safelyDeleteToken() {
    try {
      FirebaseMessaging.deleteToken()
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      // best-effort
    }
  }
}
