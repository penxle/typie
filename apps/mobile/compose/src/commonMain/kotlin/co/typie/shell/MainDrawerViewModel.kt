package co.typie.shell

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.MainDrawer_CreateSite_Mutation
import co.typie.graphql.MainDrawer_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildSite
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.CreateSiteInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result

class MainDrawerViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      MainDrawer_Query()
    }

  var isCreatingSite by mutableStateOf(false)
  var pendingCreatedSiteId by mutableStateOf<String?>(null)
    private set

  suspend fun createSite(name: String): Result<Unit, Nothing> {
    if (isCreatingSite) {
      return Result.Ok(Unit)
    }

    isCreatingSite = true
    return result<Unit, Nothing> {
        val data =
          Apollo.executeMutation(
            MainDrawer_CreateSite_Mutation(
              input = CreateSiteInput(name = name.trim().ifBlank { "새 스페이스" })
            )
          )

        pendingCreatedSiteId = data.createSite.id
        query.refetch()
      }
      .also { isCreatingSite = false }
  }

  fun consumePendingCreatedSiteSelection(siteId: String) {
    if (pendingCreatedSiteId == siteId) {
      pendingCreatedSiteId = null
    }
  }
}

private fun placeholderData() =
  MainDrawer_Query.Data(PlaceholderResolver) {
    me = buildUser {
      name = text(3..8)
      email = text(5..10) + "@example.com"
      sites = List(1) { buildSite { name = text(5..10) } }
    }
  }
