package co.typie.screen.folder

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.graphql.FolderScreen_Query
import co.typie.graphql.GraphQLViewModel
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class FolderViewModel : GraphQLViewModel() {
  var entityId by mutableStateOf("")

  val query = watchQuery(
    skip = { entityId.isBlank() },
  ) {
    FolderScreen_Query(entityId = entityId)
  }

  fun refetch() {
    if (entityId.isNotBlank()) {
      query.refetch()
    }
  }
}
