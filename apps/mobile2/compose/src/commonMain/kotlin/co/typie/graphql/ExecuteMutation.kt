package co.typie.graphql

import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Mutation
import com.apollographql.cache.normalized.optimisticUpdates

suspend fun <D : Mutation.Data> ApolloClient.executeMutation(mutation: Mutation<D>): D {
  val response = mutation(mutation).execute()
  val graphError = response.errors?.firstOrNull()

  if (response.exception != null) {
    throw response.exception!!
  } else if (graphError != null) {
    val type = graphError.extensions?.get("type") as? String
    if (type == "TypieError") {
      val code = graphError.extensions?.get("code") as String
      val message = (graphError.extensions?.get("message") as? String) ?: graphError.message
      throw TypieError(code = code, message = message)
    } else {
      throw Exception(graphError.message)
    }
  } else {
    return response.dataOrThrow()
  }
}

suspend fun <D : Mutation.Data> ApolloClient.executeMutation(
  mutation: Mutation<D>,
  optimisticUpdate: D,
): D {
  val response = mutation(mutation).optimisticUpdates(optimisticUpdate).execute()
  val graphError = response.errors?.firstOrNull()

  if (response.exception != null) {
    throw response.exception!!
  } else if (graphError != null) {
    val type = graphError.extensions?.get("type") as? String
    if (type == "TypieError") {
      val code = graphError.extensions?.get("code") as String
      val message = (graphError.extensions?.get("message") as? String) ?: graphError.message
      throw TypieError(code = code, message = message)
    } else {
      throw Exception(graphError.message)
    }
  } else {
    return response.dataOrThrow()
  }
}
