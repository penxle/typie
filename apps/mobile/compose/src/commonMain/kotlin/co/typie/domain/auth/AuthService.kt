package co.typie.domain.auth

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.Konfig
import co.typie.editor.sync.ActiveSyncEngines
import co.typie.editor.sync.catchingNonCancellation
import co.typie.editor.sync.orphanSweeper
import co.typie.graphql.Apollo
import co.typie.network.Http
import co.typie.storage.Vault
import com.apollographql.cache.normalized.apolloStore
import io.ktor.client.call.body
import io.ktor.client.plugins.ClientRequestException
import io.ktor.client.plugins.RedirectResponseException
import io.ktor.client.plugins.expectSuccess
import io.ktor.client.request.cookie
import io.ktor.client.request.forms.submitForm
import io.ktor.client.request.get
import io.ktor.client.request.parameter
import io.ktor.http.HttpHeaders
import io.ktor.http.Url
import io.ktor.http.parameters
import io.ktor.utils.io.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

object AuthService {
  private val mutex = Mutex()

  var state by mutableStateOf<AuthState>(AuthState.Unauthenticated)
    private set

  suspend fun login(sessionToken: String) {
    mutex.withLock {
      try {
        authenticate(sessionToken)
      } catch (e: InvalidCredentialsException) {
        unauthenticate()
        throw e
      }
    }
  }

  suspend fun renew() {
    mutex.withLock {
      val sessionToken = Vault.authTokens?.sessionToken
      if (sessionToken == null) {
        state = AuthState.Unauthenticated
        return@withLock
      }

      try {
        authenticate(sessionToken)
      } catch (e: InvalidCredentialsException) {
        unauthenticate()
        throw e
      }
    }
  }

  suspend fun logout() {
    withContext(Dispatchers.Main) {
      catchingNonCancellation { ActiveSyncEngines.flushAll() }
      ActiveSyncEngines.stopAll()
      catchingNonCancellation {
        orphanSweeper.sweep(includeOpenDocuments = true, deleteOnSuccess = true)
      }
    }

    mutex.withLock {
      val sessionToken = Vault.authTokens?.sessionToken
      if (sessionToken != null) {
        try {
          Http.get("${Konfig.AUTH_URL}/logout") {
            expectSuccess = false
            parameter("redirect_uri", "typie:///")
            cookie("typie-st", sessionToken)
          }
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          // best effort
        }
      }

      unauthenticate()
    }
  }

  private suspend fun authenticate(sessionToken: String) {
    val accessToken = exchangeToken(sessionToken)

    Vault.authTokens = AuthTokens(sessionToken = sessionToken, accessToken = accessToken)
    state = AuthState.Authenticated(Vault.authTokens!!)
  }

  private suspend fun exchangeToken(sessionToken: String): String {
    val code =
      try {
        Http.get("${Konfig.AUTH_URL}/authorize") {
          parameter("response_type", "code")
          parameter("redirect_uri", "typie:///authorize")
          parameter("client_id", Konfig.OIDC_CLIENT_ID)
          parameter("prompt", "none")
          cookie("typie-st", sessionToken)
        }

        error("/authorize: expected redirect response")
      } catch (e: RedirectResponseException) {
        val url =
          e.response.headers[HttpHeaders.Location]?.let { Url(it) }
            ?: error("/authorize: No Location header in redirect response")

        val error = url.parameters["error"]
        if (error != null) {
          if (error == "login_required") {
            throw InvalidCredentialsException()
          } else {
            error("/authorize: $error")
          }
        }

        url.parameters["code"] ?: error("/authorize: No code in redirect response")
      }

    return try {
      val response =
        Http.submitForm(
          url = "${Konfig.AUTH_URL}/token",
          formParameters =
            parameters {
              append("code", code)
              append("grant_type", "authorization_code")
              append("redirect_uri", "typie:///authorize")
              append("client_id", Konfig.OIDC_CLIENT_ID)
              append("client_secret", Konfig.OIDC_CLIENT_SECRET)
            },
        )

      response.body<TokenResponse>().accessToken
    } catch (e: ClientRequestException) {
      val error = e.response.body<TokenError>().error
      if (error == "invalid_grant") {
        throw InvalidCredentialsException()
      } else {
        error("/token: $error")
      }
    }
  }

  private suspend fun unauthenticate() {
    Vault.authTokens = null
    state = AuthState.Unauthenticated

    Apollo.apolloStore.clearAll()
  }

  @Serializable
  private data class TokenResponse(@SerialName("access_token") val accessToken: String)

  @Serializable private data class TokenError(val error: String)

  class InvalidCredentialsException : Exception()
}
