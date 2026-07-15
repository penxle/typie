package co.typie.network

import com.apollographql.apollo.exception.ApolloNetworkException
import com.apollographql.apollo.exception.ApolloOfflineException
import kotlinx.io.IOException

fun Throwable.isRecoverableNetworkError(): Boolean =
  this is ApolloNetworkException || this is ApolloOfflineException || this is IOException
