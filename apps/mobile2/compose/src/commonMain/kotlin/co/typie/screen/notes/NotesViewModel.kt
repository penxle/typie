package co.typie.screen.notes

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.NotesScreen_Query
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class NotesViewModel : GraphQLViewModel() {
  val query = watchQuery { NotesScreen_Query() }
}
