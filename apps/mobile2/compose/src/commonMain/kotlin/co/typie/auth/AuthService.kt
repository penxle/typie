package co.typie.auth

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.graphql.AuthService_Query
import co.typie.service.SiteService
import co.typie.storage.Vault
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.cache.normalized.FetchPolicy
import com.apollographql.apollo.cache.normalized.apolloStore
import com.apollographql.apollo.cache.normalized.fetchPolicy
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.request.forms.submitForm
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.parameter
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.http.parameters
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.koin.core.annotation.Single
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

@Single(createdAtStart = true)
class AuthService(
  private val vault: Vault,
  private val httpClient: HttpClient,
  private val siteService: SiteService,
) : KoinComponent {
  private val apolloClient: ApolloClient by inject()
  var tokens: AuthTokens? by vault("tokens", null)
    private set

  private val mutex = Mutex()

  private val _state = MutableStateFlow<AuthState>(AuthState.Initializing)
  val state: StateFlow<AuthState> = _state

  init {
    CoroutineScope(Dispatchers.Default).launch {
      initialize()
    }
  }

  private suspend fun initialize() {
    val currentTokens = tokens
    if (currentTokens == null) {
      _state.value = AuthState.Unauthenticated
      return
    }

    try {
      refreshTokensInternal(currentTokens.sessionToken)
    } catch (e: Exception) {
      Logger.e(e) { "Failed to refresh tokens" }
      if (tokens?.sessionToken != null) {
        _state.value = AuthState.Offline
      } else {
        _state.value = AuthState.Unauthenticated
      }
    }
  }

  suspend fun login(sessionToken: String) {
    if (tokens?.sessionToken == sessionToken && _state.value is AuthState.Authenticated) {
      return
    }

    mutex.withLock {
      try {
        refreshTokensInternal(sessionToken)
      } catch (e: Exception) {
        Logger.e(e) { "Failed to refresh tokens" }
        tokens = null
        _state.value = AuthState.Unauthenticated
      }
    }
  }

  suspend fun refreshTokens(): String? = mutex.withLock {
    val currentSessionToken = tokens?.sessionToken ?: return@withLock null

    try {
      refreshTokensInternal(currentSessionToken)
      return@withLock tokens?.accessToken
    } catch (e: Exception) {
      Logger.e(e) { "Failed to refresh tokens" }
      tokens = null
      _state.value = AuthState.Unauthenticated
      return@withLock null
    }
  }

  suspend fun logout() {
    val currentSessionToken = tokens?.sessionToken
    if (currentSessionToken != null) {
      try {
        httpClient.get("${Konfig.AUTH_URL}/logout") {
          parameter("redirect_uri", "typie:///")
          header("Cookie", "typie-st=$currentSessionToken")
        }
      } catch (_: Exception) {
        // best effort
      }
    }

    tokens = null
    apolloClient.apolloStore.clearAll()
    _state.value = AuthState.Unauthenticated
  }

  suspend fun retry() {
    _state.value = AuthState.Initializing
    initialize()
  }

  private suspend fun refreshTokensInternal(sessionToken: String) {
    val accessToken = exchangeToken(sessionToken)
    validateToken(accessToken)
    tokens = AuthTokens(sessionToken = sessionToken, accessToken = accessToken)
    ensureValidSiteId()
    _state.value = AuthState.Authenticated
  }

  private suspend fun ensureValidSiteId() {
    val data = apolloClient.query(AuthService_Query())
      .fetchPolicy(FetchPolicy.NetworkOnly)
      .execute().dataOrThrow()
    val siteIds = data.me.sites.map { it.id }
    if (siteIds.isNotEmpty()) {
      val currentSiteId = siteService.siteId
      if (currentSiteId.isEmpty() || currentSiteId !in siteIds) {
        siteService.siteId = siteIds.first()
      }
    }
  }

  private suspend fun exchangeToken(sessionToken: String): String {
    val authorizeResponse = httpClient.get("${Konfig.AUTH_URL}/authorize") {
      parameter("response_type", "code")
      parameter("redirect_uri", "typie:///authorize")
      parameter("client_id", Konfig.OIDC_CLIENT_ID)
      parameter("prompt", "none")
      header("Cookie", "typie-st=$sessionToken")
    }

    val location = authorizeResponse.headers["Location"]
      ?: error("No Location header in authorize response")

    val locationUri = io.ktor.http.Url(location)
    val authorizeError = locationUri.parameters["error"]
    if (authorizeError != null) {
      error("Authorize error: $authorizeError")
    }

    val code = locationUri.parameters["code"]
      ?: error("No code in authorize response")

    val tokenResponse = httpClient.submitForm(
      url = "${Konfig.AUTH_URL}/token",
      formParameters = parameters {
        append("code", code)
        append("grant_type", "authorization_code")
        append("redirect_uri", "typie:///authorize")
        append("client_id", Konfig.OIDC_CLIENT_ID)
        append("client_secret", Konfig.OIDC_CLIENT_SECRET)
      },
    )

    val body = tokenResponse.body<String>()
    val json = Json.parseToJsonElement(body).jsonObject
    return json["access_token"]?.jsonPrimitive?.content
      ?: error("No access_token in token response")
  }

  private suspend fun validateToken(accessToken: String) {
    val response = httpClient.post("${Konfig.API_URL}/graphql") {
      header("Authorization", "Bearer $accessToken")
      contentType(ContentType.Application.Json)
      setBody("""{"query":"{ me { id } }"}""")
    }

    val body = response.body<String>()
    val json = Json.parseToJsonElement(body).jsonObject
    val data = json["data"]?.jsonObject
    val me = data?.get("me")

    if (me == null || me.toString() == "null") {
      error("Invalid access token")
    }
  }
}
