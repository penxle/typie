package co.typie.screen.notes

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.NotesScreen_Query
import co.typie.graphql.watchQuery
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class NotesViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope) { NotesScreen_Query() }
}
