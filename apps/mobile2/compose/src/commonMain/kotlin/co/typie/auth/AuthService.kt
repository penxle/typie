package co.typie.auth

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.dev.SimulatedNetworkFailureException
import co.typie.service.SiteService
import co.typie.startup.AuthStartupHandle
import co.typie.storage.Vault
import com.apollographql.apollo.ApolloClient
import com.apollographql.cache.normalized.apolloStore
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.ResponseException
import io.ktor.client.network.sockets.ConnectTimeoutException
import io.ktor.client.network.sockets.SocketTimeoutException
import io.ktor.client.request.forms.submitForm
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.parameter
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.http.parameters
import io.ktor.util.network.UnresolvedAddressException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.io.IOException
import org.koin.core.annotation.Single
import org.koin.core.component.KoinComponent
import org.koin.core.component.inject

internal data class AuthenticatedUserContext(
  val userId: String,
  val siteIds: List<String>,
)

internal fun parseAuthenticatedUserContextResponse(body: String): AuthenticatedUserContext {
  val json = Json.parseToJsonElement(body).jsonObject
  val data = json["data"]?.jsonObject ?: error("Invalid access token")
  val me = data["me"]?.takeUnless { it.toString() == "null" }?.jsonObject ?: error("Invalid access token")

  val userId = me["id"]?.jsonPrimitive?.content ?: error("Invalid user id")
  val siteIds = me["sites"]?.jsonArray?.map { site ->
    site.jsonObject["id"]?.jsonPrimitive?.content ?: error("Invalid site id")
  }.orEmpty()

  return AuthenticatedUserContext(userId = userId, siteIds = siteIds)
}

internal enum class AuthFailureDisposition {
  ClearSession,
  Offline,
}

internal fun classifyAuthFailure(error: Throwable): AuthFailureDisposition {
  val causes = generateSequence(error) { it.cause }.toList()

  val responseException = causes.filterIsInstance<ResponseException>().firstOrNull()
  if (responseException != null) {
    return if (responseException.response.status.value == 401) {
      AuthFailureDisposition.ClearSession
    } else {
      AuthFailureDisposition.Offline
    }
  }

  if (causes.any { cause ->
      cause is IOException ||
        cause is ConnectTimeoutException ||
        cause is SocketTimeoutException ||
        cause is UnresolvedAddressException ||
        cause is SimulatedNetworkFailureException ||
        cause.message == "Simulated network failure"
    }) {
    return AuthFailureDisposition.Offline
  }

  return AuthFailureDisposition.ClearSession
}

@Single(binds = [AuthStartupHandle::class])
class AuthService(
  private val vault: Vault,
  private val httpClient: HttpClient,
  private val siteService: SiteService,
) : KoinComponent, AuthStartupHandle {
  private val apolloClient: ApolloClient by inject()
  var tokens: AuthTokens? by vault("tokens", null)
    private set

  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private val mutex = Mutex()
  private var started = false

  private val _state = MutableStateFlow<AuthState>(AuthState.Initializing)
  val state: StateFlow<AuthState> = _state

  fun loginAsync(sessionToken: String) {
    scope.launch {
      login(sessionToken)
    }
  }

  fun startAsync() {
    scope.launch {
      start()
    }
  }

  fun retryAsync() {
    scope.launch {
      retry()
    }
  }

  override suspend fun start() {
    mutex.withLock {
      if (started) return@withLock
      started = true
      Logger.i { "Auth startup: begin." }
      initializeLocked()
    }
  }

  private suspend fun initializeLocked() {
    val currentTokens = tokens
    if (currentTokens == null) {
      Logger.i { "Auth startup: no stored session token." }
      _state.value = AuthState.Unauthenticated
      return
    }

    try {
      refreshTokensInternal(currentTokens.sessionToken)
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      handleRefreshFailure(error = e, sessionToken = currentTokens.sessionToken)
    }
  }

  suspend fun login(sessionToken: String) {
    mutex.withLock {
      started = true
      if (tokens?.sessionToken == sessionToken && _state.value is AuthState.Authenticated) {
        return@withLock
      }

      val currentTokens = tokens
      if (currentTokens?.sessionToken != sessionToken) {
        tokens = AuthTokens(sessionToken = sessionToken)
      }

      try {
        refreshTokensInternal(sessionToken)
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        handleRefreshFailure(error = e, sessionToken = sessionToken)
      }
    }
  }

  suspend fun refreshTokens(): String? = mutex.withLock {
    started = true
    val currentSessionToken = tokens?.sessionToken ?: return@withLock null

    try {
      refreshTokensInternal(currentSessionToken)
      return@withLock tokens?.accessToken
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      handleRefreshFailure(error = e, sessionToken = currentSessionToken)
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

    clearSession()
  }

  suspend fun clearSession() {
    tokens = null
    siteService.clearCurrentUser()
    apolloClient.apolloStore.clearAll()
    _state.value = AuthState.Unauthenticated
  }

  private suspend fun retry() {
    mutex.withLock {
      started = true
      _state.value = AuthState.Initializing
      initializeLocked()
    }
  }

  private suspend fun handleRefreshFailure(
    error: Exception,
    sessionToken: String?,
  ) {
    Logger.e(error) { "Failed to refresh tokens" }

    if (sessionToken == null) {
      clearSession()
      return
    }

    when (classifyAuthFailure(error)) {
      AuthFailureDisposition.ClearSession -> clearSession()
      AuthFailureDisposition.Offline -> _state.value = AuthState.Offline
    }
  }

  private suspend fun refreshTokensInternal(sessionToken: String) {
    val accessToken = exchangeToken(sessionToken)
    val authenticatedUserContext = fetchAuthenticatedUserContext(accessToken)
    tokens = AuthTokens(sessionToken = sessionToken, accessToken = accessToken)
    ensureValidSiteId(authenticatedUserContext)
    _state.value = AuthState.Authenticated
    Logger.i { "Auth startup: authenticated user=${authenticatedUserContext.userId} sites=${authenticatedUserContext.siteIds.size}." }
  }

  private fun ensureValidSiteId(authenticatedUserContext: AuthenticatedUserContext) {
    siteService.bindUser(
      userId = authenticatedUserContext.userId,
      availableSiteIds = authenticatedUserContext.siteIds,
    )
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

  private suspend fun fetchAuthenticatedUserContext(accessToken: String): AuthenticatedUserContext {
    val response = httpClient.post("${Konfig.API_URL}/graphql") {
      header("Authorization", "Bearer $accessToken")
      contentType(ContentType.Application.Json)
      setBody("""{"query":"{ me { id sites { id } } }"}""")
    }

    val body = response.body<String>()
    return parseAuthenticatedUserContextResponse(body)
  }
}
