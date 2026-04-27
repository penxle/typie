package co.typie.graphql

import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import com.apollographql.apollo.api.http.HttpRequest
import com.apollographql.apollo.api.http.HttpResponse
import com.apollographql.apollo.network.http.HttpInterceptor
import com.apollographql.apollo.network.http.HttpInterceptorChain

object DeviceInterceptor : HttpInterceptor {
  override suspend fun intercept(request: HttpRequest, chain: HttpInterceptorChain): HttpResponse {
    val info = PlatformModule.deviceInfo.retrieve()
    val newRequest =
      request
        .newBuilder()
        .addHeader("X-Device-Id", Preference.deviceId)
        .addHeader("X-Device-Name", info.model)
        .addHeader("X-Device-Platform", info.osName.uppercase())
        .build()
    return chain.proceed(newRequest)
  }
}
