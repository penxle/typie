package co.typie.graphql

import co.typie.Konfig
import co.typie.auth.AuthInterceptor
import com.apollographql.apollo.ApolloClient
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.api.CacheKey
import com.apollographql.cache.normalized.api.IdCacheKeyGenerator
import com.apollographql.cache.normalized.api.IdCacheResolver
import com.apollographql.cache.normalized.fetchPolicy
import com.apollographql.cache.normalized.memory.MemoryCacheFactory
import com.apollographql.cache.normalized.normalizedCache
import com.apollographql.ktor.ktorClient
import io.ktor.client.HttpClient
import org.koin.core.annotation.Single

@Single
fun apolloClient(httpClient: HttpClient, authInterceptor: AuthInterceptor): ApolloClient =
  ApolloClient.Builder()
    .serverUrl("${Konfig.API_URL}/graphql")
    .ktorClient(httpClient)
    .fetchPolicy(FetchPolicy.CacheAndNetwork)
    .normalizedCache(
      MemoryCacheFactory(maxSizeBytes = 10 * 1024 * 1024),
      cacheKeyGenerator = IdCacheKeyGenerator(keyScope = CacheKey.Scope.SERVICE),
      cacheResolver = IdCacheResolver(keyScope = CacheKey.Scope.SERVICE),
    )
    .addHttpInterceptor(authInterceptor)
    .build()

