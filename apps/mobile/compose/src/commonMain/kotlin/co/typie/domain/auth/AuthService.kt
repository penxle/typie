package co.typie.domain.auth

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.Konfig
import co.typie.domain.subscription.shouldDiscardEntitlementCache
import co.typie.editor.sync.ActiveDocumentEditingSessions
import co.typie.editor.sync.catchingNonCancellation
import co.typie.editor.sync.orphanSweeper
import co.typie.editor.sync.ws.SyncWs
import co.typie.graphql.Apollo
import co.typie.graphql.closeApolloSubscriptionConnection
import co.typie.network.Http
import co.typie.storage.Preference
import co.typie.storage.Vault
import com.apollographql.cache.normalized.apolloStore
import io.ktor.client.call.body
import io.ktor.client.plugins.ClientRequestException
import io.ktor.client.plugins.RedirectResponseException
import io.ktor.client.plugins.expectSuccess
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.cookie
import io.ktor.client.request.forms.submitForm
import io.ktor.client.request.get
import io.ktor.client.request.parameter
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.Url
import io.ktor.http.contentType
import io.ktor.http.parameters
import io.ktor.serialization.ContentConvertException
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
      catchingNonCancellation { ActiveDocumentEditingSessions.flushSyncAll() }
      ActiveDocumentEditingSessions.stopAll()
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

    val previousTokens = Vault.authTokens
    val previousSessionToken = previousTokens?.sessionToken

    val reusableUserId = previousTokens?.userId?.takeIf { previousSessionToken == sessionToken }
    val userId =
      if (reusableUserId != null) {
        Preference.switchUser(reusableUserId)
        reusableUserId
      } else {
        val me = fetchMe(accessToken)
        Preference.switchUser(me.id)
        resolveActiveSiteId(Preference.siteId, me.sites.map { it.id })?.let {
          Preference.siteId = it
        }
        me.id
      }

    // 인증 전체(토큰 교환 + me 조회)가 성공한 뒤에만 소거한다 — 실패 시 부분 상태를 남기지 않는다.
    if (shouldDiscardEntitlementCache(previousSessionToken, sessionToken)) {
      Preference.entitlementCache = null
    }

    Vault.authTokens =
      AuthTokens(sessionToken = sessionToken, accessToken = accessToken, userId = userId)
    state = AuthState.Authenticated(Vault.authTokens!!)

    // 로그아웃을 거치지 않고 세션이 바뀌는 경로 방어. state 갱신 후에 끊어야 재연결이 새 유저 티켓을 받는다.
    if (previousSessionToken != null && previousSessionToken != sessionToken) {
      closeApolloSubscriptionConnection()
      SyncWs.onSessionChanged()
    }
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
      val error =
        runCatching { e.response.body<TokenError>().error }.getOrNull()
          ?: error("/token: HTTP ${e.response.status.value}")
      if (error == "invalid_grant") {
        throw InvalidCredentialsException()
      } else {
        error("/token: $error")
      }
    } catch (e: ContentConvertException) {
      error("/token: malformed response (${e.message})")
    }
  }

  private suspend fun unauthenticate() {
    Vault.authTokens = null
    // 권한 캐시는 세션에 귀속된다 — 남겨두면 다음 로그인 유저가 앞 유저의 권한을 본다.
    Preference.entitlementCache = null
    Preference.switchUser(null)
    state = AuthState.Unauthenticated

    Apollo.apolloStore.clearAll()
    // 두 WS 모두 hello/connectionPayload 시점 유저로 인증이 고정된다 — 신원이 끝나면 즉시 끊는다.
    closeApolloSubscriptionConnection()
    SyncWs.onSessionChanged()
  }

  private suspend fun fetchMe(accessToken: String): Me {
    val response =
      Http.post("${Konfig.API_URL}/graphql") {
        contentType(ContentType.Application.Json)
        bearerAuth(accessToken)
        setBody(GraphQLRequest(query = "query AuthService_Me { me { id sites { id } } }"))
      }
    val body = response.body<MeResponse>()
    return body.data?.me ?: error("/graphql me: ${body.errors.firstOrNull()?.message ?: "no data"}")
  }

  @Serializable private data class GraphQLRequest(val query: String)

  @Serializable
  private data class MeResponse(val data: MeData? = null, val errors: List<MeError> = emptyList())

  @Serializable private data class MeError(val message: String? = null)

  @Serializable private data class MeData(val me: Me? = null)

  @Serializable private data class Me(val id: String, val sites: List<MeSite>)

  @Serializable private data class MeSite(val id: String)

  @Serializable
  private data class TokenResponse(@SerialName("access_token") val accessToken: String)

  @Serializable private data class TokenError(val error: String)

  class InvalidCredentialsException : Exception()
}
