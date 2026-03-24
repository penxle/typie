package co.typie.graphql

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Mutation
import com.apollographql.apollo.api.Query
import org.koin.core.component.KoinComponent
import org.koin.core.component.get

abstract class GraphQLViewModel : ViewModel(), KoinComponent {
  val apolloClient: ApolloClient = get()

  fun <D : Query.Data> watchQuery(
    onInitialData: ((D) -> Unit)? = null,
    skip: () -> Boolean = { false },
    resetOnChange: Boolean = true,
    query: () -> Query<D>,
  ): WatchQuery<D, D?> {
    return WatchQuery(
      viewModelScope,
      apolloClient,
      query,
      onInitialData = onInitialData,
      skip = skip,
      resetOnChange = resetOnChange
    )
  }

  fun <D : Query.Data> watchQuery(
    placeholderData: D,
    onInitialData: ((D) -> Unit)? = null,
    skip: () -> Boolean = { false },
    resetOnChange: Boolean = true,
    query: () -> Query<D>,
  ): WatchQuery<D, D> {
    return WatchQuery(
      viewModelScope,
      apolloClient,
      query,
      placeholderData,
      onInitialData,
      skip,
      resetOnChange
    )
  }

  suspend fun <D : Mutation.Data> executeMutation(
    mutation: Mutation<D>,
  ): D {
    return apolloClient.executeMutation(mutation)
  }
}
