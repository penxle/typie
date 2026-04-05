package co.typie.graphql

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.auth.AuthService
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.request.header
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.koin.core.annotation.Single

private const val CreateWsSessionMutation =
  """{"query":"mutation CreateWsSession_Mutation { createWsSession }"}"""

internal fun parseCreateWsSessionResponse(body: String): String {
  val data = Json.parseToJsonElement(body).jsonObject["data"]?.jsonObject
    ?: throw IllegalStateException("Invalid createWsSession response")

  return data["createWsSession"]?.jsonPrimitive?.contentOrNull
    ?: throw IllegalStateException("Missing websocket session token")
}

@Single
class WebSocketSessionService(
  private val authService: AuthService,
  private val httpClient: HttpClient,
) {
  suspend fun createConnectionPayload(): Map<String, Any?> = mapOf("session" to createSession())

  suspend fun createSession(): String {
    val accessToken = authService.tokens?.accessToken ?: authService.refreshTokens()
      ?: error("Missing access token for websocket session")

    return try {
      requestSession(accessToken)
    } catch (firstError: Exception) {
      val refreshedAccessToken = authService.refreshTokens()
      if (refreshedAccessToken == null || refreshedAccessToken == accessToken) {
        throw firstError
      }

      Logger.e(firstError) { "Retrying websocket session creation with refreshed token" }
      requestSession(refreshedAccessToken)
    }
  }

  private suspend fun requestSession(accessToken: String): String {
    val response = httpClient.post("${Konfig.API_URL}/graphql") {
      header("Authorization", "Bearer $accessToken")
      contentType(ContentType.Application.Json)
      setBody(CreateWsSessionMutation)
    }

    return parseCreateWsSessionResponse(response.body())
  }
}
