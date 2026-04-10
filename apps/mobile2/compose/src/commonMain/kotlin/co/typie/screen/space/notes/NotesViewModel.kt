package co.typie.screen.space.notes

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.NotesScreen_Query
import co.typie.graphql.Apollo
import co.typie.graphql.watchQuery

class NotesViewModel : ViewModel() {

  val query = Apollo.watchQuery(scope = viewModelScope) { NotesScreen_Query() }
}
