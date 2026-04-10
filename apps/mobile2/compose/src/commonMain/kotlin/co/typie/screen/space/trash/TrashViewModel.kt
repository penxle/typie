package co.typie.screen.space.trash

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.TrashScreen_PurgeEntities_Mutation
import co.typie.graphql.TrashScreen_RecoverEntity_Mutation
import co.typie.graphql.TrashScreen_WithEntityId_Query
import co.typie.graphql.TrashScreen_WithSiteId_Query
import co.typie.graphql.executeMutation
import co.typie.graphql.type.PurgeEntitiesInput
import co.typie.graphql.type.RecoverEntityInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.service.SiteService
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
internal class TrashViewModel(
  val siteService: SiteService,
  private val apolloClient: ApolloClient,
) : ViewModel() {
  var entityId: String? by mutableStateOf(null)

  val siteQuery = apolloClient.watchQuery(
    scope = viewModelScope,
    skip = { entityId != null },
  ) {
    TrashScreen_WithSiteId_Query(siteId = siteService.siteId)
  }

  val entityQuery = apolloClient.watchQuery(
    scope = viewModelScope,
    skip = { entityId == null },
  ) {
    TrashScreen_WithEntityId_Query(entityId = requireNotNull(entityId))
  }

  fun refetch() {
    if (entityId == null) {
      siteQuery.refetch()
    } else {
      entityQuery.refetch()
    }
  }

  suspend fun recoverEntity(item: TrashItem): Result<String, Nothing> {
    return result {
      val data = apolloClient.executeMutation(
        TrashScreen_RecoverEntity_Mutation(
          input = RecoverEntityInput(entityId = item.id),
        ),
      )
      "\"${trashRecoveryPath(data.recoverEntity)}\" ${item.type.label}를 복원했어요"
    }
  }

  suspend fun purgeEntities(entityIds: List<String>): Result<Unit, Nothing> {
    return result {
      apolloClient.executeMutation(
        TrashScreen_PurgeEntities_Mutation(
          input = PurgeEntitiesInput(entityIds = entityIds),
        ),
      )
    }
  }
}

private fun trashRecoveryPath(
  entity: TrashScreen_RecoverEntity_Mutation.RecoverEntity,
): String {
  val segments = buildList {
    addAll(entity.ancestors.mapNotNull { it.node.onFolder?.name })
    add(entity.node.onFolder?.name ?: entity.node.onDocument?.title ?: "삭제된 항목")
  }

  return segments.joinToString(" › ")
}
