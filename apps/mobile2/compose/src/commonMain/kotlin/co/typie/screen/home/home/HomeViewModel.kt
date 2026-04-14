package co.typie.screen.home.home

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.EntityContainer_CreateDocument_Mutation
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.CreateDocumentInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import co.typie.storage.Preference
import co.typie.storage.Preference.siteId

class HomeViewModel : ViewModel() {
  var isCreatingDocument by mutableStateOf(false)
    private set

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { Preference.siteId == null },
    ) {
      HomeScreen_Query(siteId = Preference.siteId!!)
    }

  suspend fun createDocument(): Result<String, Nothing> =
    loading({ isCreatingDocument = it }) {
      val response =
        Apollo.executeMutation(
          EntityContainer_CreateDocument_Mutation(CreateDocumentInput(siteId = Preference.siteId!!))
        )

      response.createDocument.entity.id
    }
}

private fun placeholderData() =
  HomeScreen_Query.Data(PlaceholderResolver) {
    site = buildSite { name = text(5..10) }
    me = buildUser {
      recentlyViewedEntities =
        List(15) {
          buildEntity {
            node =
              if (it < 5) {
                buildFolder { name = text(5..10) }
              } else {
                buildDocument {
                  title = text(5..20)
                  subtitle = if (it % 2 == 0) text(4..12) else null
                  excerpt = text(20..30)
                }
              }
          }
        }
    }
  }
