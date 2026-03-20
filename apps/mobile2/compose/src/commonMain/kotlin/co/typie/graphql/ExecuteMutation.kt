package co.typie.graphql

import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Mutation

sealed class MutationResult<out T> {
  data class Success<T>(val data: T) : MutationResult<T>()
  data class Failure(val error: TypieError) : MutationResult<Nothing>()
  data class Error(val exception: Exception) : MutationResult<Nothing>()
}

suspend fun <D : Mutation.Data> ApolloClient.executeMutation(
  mutation: Mutation<D>,
): MutationResult<D> {
  return try {
    val response = mutation(mutation).execute()
    val gqlError = response.errors?.firstOrNull()

    if (response.exception != null) {
      MutationResult.Error(response.exception!!)
    } else if (gqlError != null) {
      val type = gqlError.extensions?.get("type") as? String
      if (type == "TypieError") {
        val code = gqlError.extensions?.get("code") as String
        val message = gqlError.extensions?.get("message") as String?
        MutationResult.Failure(TypieError(code = code, message = message))
      } else {
        MutationResult.Error(Exception(gqlError.message))
      }
    } else {
      MutationResult.Success(response.dataOrThrow())
    }
  } catch (e: Exception) {
    MutationResult.Error(e)
  }
}
