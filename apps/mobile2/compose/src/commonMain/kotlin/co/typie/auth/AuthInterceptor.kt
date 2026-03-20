package co.typie.auth

import com.apollographql.apollo.api.http.HttpRequest
import com.apollographql.apollo.api.http.HttpResponse
import com.apollographql.apollo.network.http.HttpInterceptor
import com.apollographql.apollo.network.http.HttpInterceptorChain
import org.koin.core.annotation.Single

@Single
class AuthInterceptor(
  private val authService: AuthService,
) : HttpInterceptor {

  override suspend fun intercept(
    request: HttpRequest,
    chain: HttpInterceptorChain,
  ): HttpResponse {
    val authedRequest = authService.tokens?.accessToken?.let { token ->
      request.newBuilder()
        .addHeader("Authorization", "Bearer $token")
        .build()
    } ?: request

    val response = chain.proceed(authedRequest)

    response.headers.firstOrNull {
      it.name.equals(
        "set-cookie",
        ignoreCase = true
      ) && it.value.startsWith("typie-st=")
    }
      ?.let { cookie ->
        val sessionToken = cookie.value.substringAfter("typie-st=").substringBefore(";")
        authService.login(sessionToken)
      }

    if (response.statusCode == 401) {
      val newToken = authService.refreshTokens()
      if (newToken != null) {
        val retryRequest = request.newBuilder()
          .addHeader("Authorization", "Bearer $newToken")
          .build()
        return chain.proceed(retryRequest)
      }
    }

    return response
  }
}
