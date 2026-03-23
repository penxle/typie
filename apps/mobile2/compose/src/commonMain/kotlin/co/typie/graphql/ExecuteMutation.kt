package co.typie.graphql

import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Mutation

suspend fun <D : Mutation.Data> ApolloClient.executeMutation(
  mutation: Mutation<D>,
): D {
  val response = mutation(mutation).execute()
  val gqlError = response.errors?.firstOrNull()

  if (response.exception != null) {
    throw response.exception!!
  } else if (gqlError != null) {
    val type = gqlError.extensions?.get("type") as? String
    if (type == "TypieError") {
      val code = gqlError.extensions?.get("code") as String
      val message = gqlError.extensions?.get("message") as String?
      throw TypieError(code = code, message = message)
    } else {
      throw Exception(gqlError.message)
    }
  } else {
    return response.dataOrThrow()
  }
}
