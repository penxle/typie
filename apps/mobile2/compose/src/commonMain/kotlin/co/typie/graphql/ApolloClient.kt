package co.typie.graphql

import co.typie.Konfig
import co.typie.auth.AuthInterceptor
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.cache.normalized.FetchPolicy
import com.apollographql.apollo.cache.normalized.api.MemoryCacheFactory
import com.apollographql.apollo.cache.normalized.fetchPolicy
import com.apollographql.apollo.cache.normalized.normalizedCache
import com.apollographql.ktor.ktorClient
import io.ktor.client.HttpClient
import org.koin.core.annotation.Single

@Single
fun apolloClient(httpClient: HttpClient, authInterceptor: AuthInterceptor): ApolloClient =
  ApolloClient.Builder()
    .serverUrl("${Konfig.API_URL}/graphql")
    .ktorClient(httpClient)
    .fetchPolicy(FetchPolicy.CacheAndNetwork)
    .normalizedCache(MemoryCacheFactory(maxSizeBytes = 10 * 1024 * 1024))
    .addHttpInterceptor(authInterceptor)
    .build()
