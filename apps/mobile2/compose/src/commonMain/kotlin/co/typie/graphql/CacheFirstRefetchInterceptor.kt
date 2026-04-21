package co.typie.graphql

import com.apollographql.apollo.api.ApolloRequest
import com.apollographql.apollo.api.ApolloResponse
import com.apollographql.apollo.api.Operation
import com.apollographql.apollo.interceptor.ApolloInterceptor
import com.apollographql.apollo.interceptor.ApolloInterceptorChain
import com.apollographql.cache.normalized.fetchFromCache
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emitAll
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.single

// apollo-kotlin-normalized-cache 1.0.1의 refetchPolicy(FetchPolicy.CacheFirst)는
// 내부 reset 블록이 refetchOnlyIfCached를 true로 남겨 watch 시 network fall-through가
// 동작하지 않는다. DefaultFetchPolicyInterceptor의 CacheFirst 로직을 그대로 복제하되
// onlyIfCached 체크를 생략해 partial cache miss 시에도 network로 refetch 한다.
object CacheFirstRefetchInterceptor : ApolloInterceptor {
  override fun <D : Operation.Data> intercept(
    request: ApolloRequest<D>,
    chain: ApolloInterceptorChain,
  ): Flow<ApolloResponse<D>> = flow {
    val cacheResponse = chain.proceed(request.newBuilder().fetchFromCache(true).build()).single()

    emit(cacheResponse.newBuilder().isLast(cacheResponse.exception == null).build())

    if (cacheResponse.exception == null) {
      return@flow
    }

    emitAll(chain.proceed(request))
  }
}
