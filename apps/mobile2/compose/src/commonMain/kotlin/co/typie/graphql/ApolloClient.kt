package co.typie.graphql

import co.typie.Konfig
import co.typie.network.Http
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.annotations.ApolloExperimental
import com.apollographql.apollo.api.Subscription
import com.apollographql.apollo.network.websocket.GraphQLWsProtocol
import com.apollographql.apollo.network.websocket.WebSocketNetworkTransport
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.api.CacheKey
import com.apollographql.cache.normalized.api.IdCacheKeyGenerator
import com.apollographql.cache.normalized.api.IdCacheResolver
import com.apollographql.cache.normalized.fetchPolicy
import com.apollographql.cache.normalized.memory.MemoryCacheFactory
import com.apollographql.cache.normalized.normalizedCache
import com.apollographql.ktor.http.KtorHttpEngine

@OptIn(ApolloExperimental::class)
val Apollo: ApolloClient =
  ApolloClient.Builder()
    .serverUrl("${Konfig.API_URL}/graphql")
    .httpEngine(KtorHttpEngine(Http))
    .fetchPolicy(FetchPolicy.CacheAndNetwork)
    .retryOnError { request -> request.operation is Subscription<*> }
    .subscriptionNetworkTransport(
      WebSocketNetworkTransport.Builder()
        .serverUrl("${Konfig.WS_URL}/graphql")
        .webSocketEngine(KtorWebSocketEngine)
        .wsProtocol(
          GraphQLWsProtocol(connectionPayload = { mapOf("session" to WebSocketSession.create()) })
        )
        .build()
    )
    .normalizedCache(
      MemoryCacheFactory(maxSizeBytes = 10 * 1024 * 1024),
      cacheKeyGenerator = IdCacheKeyGenerator(keyScope = CacheKey.Scope.SERVICE),
      cacheResolver = IdCacheResolver(keyScope = CacheKey.Scope.SERVICE),
      enableOptimisticUpdates = true,
      writeToCacheAsynchronously = true,
    )
    .addHttpInterceptor(AuthInterceptor)
    .build()
