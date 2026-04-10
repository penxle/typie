package co.typie.auth

import com.apollographql.apollo.api.http.HttpRequest
import com.apollographql.apollo.api.http.HttpResponse
import com.apollographql.apollo.network.http.HttpInterceptor
import com.apollographql.apollo.network.http.HttpInterceptorChain

object AuthInterceptor : HttpInterceptor {
  override suspend fun intercept(request: HttpRequest, chain: HttpInterceptorChain): HttpResponse {
    val currentAccessToken = AuthService.tokens?.accessToken

    val authedRequest =
      currentAccessToken?.let { token ->
        request.newBuilder().addHeader("Authorization", "Bearer $token").build()
      } ?: request

    val response = chain.proceed(authedRequest)

    response.headers
      .firstOrNull {
        it.name.equals("set-cookie", ignoreCase = true) && it.value.startsWith("typie-st=")
      }
      ?.let { cookie ->
        val sessionToken = cookie.value.substringAfter("typie-st=").substringBefore(";")
        AuthService.loginAsync(sessionToken)
      }

    if (response.statusCode == 401) {
      val newToken = AuthService.refreshTokens()
      if (newToken != null) {
        val retryRequest =
          request.newBuilder().addHeader("Authorization", "Bearer $newToken").build()
        return chain.proceed(retryRequest)
      }
    }

    return response
  }
}
