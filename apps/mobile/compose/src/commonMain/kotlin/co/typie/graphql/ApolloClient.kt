package co.typie.graphql

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.network.Http
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.annotations.ApolloExperimental
import com.apollographql.apollo.api.Subscription
import com.apollographql.apollo.exception.ApolloNetworkException
import com.apollographql.apollo.network.websocket.GraphQLWsProtocol
import com.apollographql.apollo.network.websocket.WebSocketNetworkTransport
import com.apollographql.cache.normalized.api.CacheKey
import com.apollographql.cache.normalized.api.IdCacheKeyGenerator
import com.apollographql.cache.normalized.api.IdCacheResolver
import com.apollographql.cache.normalized.memory.MemoryCacheFactory
import com.apollographql.cache.normalized.normalizedCache
import com.apollographql.ktor.http.KtorHttpEngine
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.CancellationException

private val subscriptionTransport: WebSocketNetworkTransport =
  WebSocketNetworkTransport.Builder()
    .serverUrl("${Konfig.WS_URL}/graphql")
    .webSocketEngine(KtorWebSocketEngine)
    .wsProtocol(
      GraphQLWsProtocol(
        connectionPayload = {
          try {
            mapOf("session" to WebSocketSession.create())
          } catch (e: CancellationException) {
            throw e
          } catch (e: Exception) {
            // 여기서 던지면 Apollo 내부의 무보호 코루틴에서 미처리 예외로 앱이 죽는다. null이면
            // 서버가 connection_init을 거부하고 정상 백오프 재시도로 흘러간다.
            Logger.w(e) { "Apollo subscription: ticket fetch failed" }
            null
          }
        }
      )
    )
    .pingInterval(30.seconds)
    .build()

/** 구독 WS는 연결 시점 세션 티켓으로 인증이 고정된다. 계정 전환 시 끊으면 retryOnError 구독들이 새 티켓으로 재연결한다. */
fun closeApolloSubscriptionConnection() {
  subscriptionTransport.closeConnection(ApolloNetworkException("session changed"))
}

@OptIn(ApolloExperimental::class)
val Apollo: ApolloClient =
  ApolloClient.Builder()
    .serverUrl("${Konfig.API_URL}/graphql")
    .httpEngine(KtorHttpEngine(Http))
    .retryOnError { request -> request.operation is Subscription<*> }
    .subscriptionNetworkTransport(subscriptionTransport)
    .normalizedCache(
      MemoryCacheFactory(maxSizeBytes = 10 * 1024 * 1024),
      cacheKeyGenerator = IdCacheKeyGenerator(keyScope = CacheKey.Scope.SERVICE),
      cacheResolver = IdCacheResolver(keyScope = CacheKey.Scope.SERVICE),
      enableOptimisticUpdates = true,
    )
    .addHttpInterceptor(AuthInterceptor)
    .addHttpInterceptor(DeviceInterceptor)
    .build()
