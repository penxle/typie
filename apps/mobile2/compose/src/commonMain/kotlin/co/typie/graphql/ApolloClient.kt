package co.typie.graphql

import co.typie.Konfig
import co.typie.auth.AuthInterceptor
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.annotations.ApolloExperimental
import com.apollographql.apollo.api.Subscription
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.api.CacheKey
import com.apollographql.cache.normalized.api.IdCacheKeyGenerator
import com.apollographql.cache.normalized.api.IdCacheResolver
import com.apollographql.cache.normalized.fetchPolicy
import com.apollographql.cache.normalized.memory.MemoryCacheFactory
import com.apollographql.cache.normalized.normalizedCache
import com.apollographql.apollo.network.ws.GraphQLWsProtocol
import com.apollographql.ktor.ktorClient
import io.ktor.client.HttpClient
import org.koin.core.annotation.Single

@OptIn(ApolloExperimental::class)
@Single
fun apolloClient(
  httpClient: HttpClient,
  authInterceptor: AuthInterceptor,
  webSocketSessionService: WebSocketSessionService,
): ApolloClient =
  ApolloClient.Builder()
    .serverUrl("${Konfig.API_URL}/graphql")
    .webSocketServerUrl("${Konfig.WS_URL}/graphql")
    .ktorClient(httpClient)
    .fetchPolicy(FetchPolicy.CacheAndNetwork)
    .retryOnError { request -> request.operation is Subscription<*> }
    .wsProtocol(
      GraphQLWsProtocol.Factory(
        connectionPayload = { webSocketSessionService.createConnectionPayload() },
      ),
    )
    .normalizedCache(
      MemoryCacheFactory(maxSizeBytes = 10 * 1024 * 1024),
      cacheKeyGenerator = IdCacheKeyGenerator(keyScope = CacheKey.Scope.SERVICE),
      cacheResolver = IdCacheResolver(keyScope = CacheKey.Scope.SERVICE),
    )
    .addHttpInterceptor(authInterceptor)
    .build()
