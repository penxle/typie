package co.typie.auth

import co.typie.Konfig
import co.typie.graphql.Apollo
import co.typie.network.Http
import co.typie.storage.vault
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
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

object AuthService {
  var tokens: AuthTokens? by vault("tokens", null)
    private set

  private val _state = MutableStateFlow<AuthState>(AuthState.Unauthenticated)
  val state: StateFlow<AuthState> = _state

  private val mutex = Mutex()

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
      val sessionToken = tokens?.sessionToken
      if (sessionToken == null) {
        _state.value = AuthState.Unauthenticated
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
    mutex.withLock {
      val sessionToken = tokens?.sessionToken
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

    tokens = AuthTokens(sessionToken = sessionToken, accessToken = accessToken)
    _state.value = AuthState.Authenticated
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
    tokens = null
    _state.value = AuthState.Unauthenticated

    Apollo.apolloStore.clearAll()
  }

  @Serializable
  private data class TokenResponse(@SerialName("access_token") val accessToken: String)

  @Serializable private data class TokenError(val error: String)

  private class InvalidCredentialsException : Exception()
}
