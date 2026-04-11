package co.typie.graphql

import co.typie.auth.AuthService
import co.typie.auth.AuthState
import com.apollographql.apollo.api.http.HttpRequest
import com.apollographql.apollo.api.http.HttpResponse
import com.apollographql.apollo.network.http.HttpInterceptor
import com.apollographql.apollo.network.http.HttpInterceptorChain
import io.ktor.http.parseServerSetCookieHeader
import kotlin.coroutines.cancellation.CancellationException

object AuthInterceptor : HttpInterceptor {
  override suspend fun intercept(request: HttpRequest, chain: HttpInterceptorChain): HttpResponse {
    val newRequest =
      when (val authState = AuthService.state) {
        is AuthState.Authenticated ->
          request
            .newBuilder()
            .addHeader("Authorization", "Bearer ${authState.tokens.accessToken}")
            .build()
        else -> request
      }

    val response = chain.proceed(newRequest)

    val sessionTokenCookie =
      response.headers
        .filter { it.name.equals("set-cookie", ignoreCase = true) }
        .map { parseServerSetCookieHeader(it.value) }
        .firstOrNull { it.name == "typie-st" }

    if (sessionTokenCookie != null) {
      try {
        AuthService.login(sessionTokenCookie.value)
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        // best effort
      }
    }

    return response
  }
}
