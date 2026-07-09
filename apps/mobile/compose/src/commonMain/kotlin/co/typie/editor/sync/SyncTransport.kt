package co.typie.editor.sync

import co.typie.graphql.Apollo
import co.typie.graphql.DocumentSync_ChangesetsUpdated_Subscription
import co.typie.graphql.DocumentSync_PullDocumentChangesets_Mutation
import co.typie.graphql.DocumentSync_PushDocumentChangesets_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.PullDocumentChangesetsInput
import co.typie.graphql.type.PushDocumentChangesetsInput
import com.apollographql.apollo.api.Optional
import com.apollographql.apollo.exception.ApolloHttpException
import com.apollographql.apollo.exception.SubscriptionOperationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

data class PullResult(
  val changesets: List<ByteArray>,
  val seq: String,
  val heads: ByteArray,
  val durableHeads: ByteArray,
  val needsReload: Boolean,
)

data class RemoteChangesetEvent(
  val changesets: List<ByteArray>,
  val seq: String,
  val heads: ByteArray,
  val durableHeads: ByteArray,
)

interface SyncTransport {
  suspend fun push(changesets: ByteArray): PushResult

  suspend fun pull(sinceSeq: String?): PullResult

  fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent>
}

fun isPermanentSyncError(error: Throwable): Boolean = isPermanentSyncError(error, mutableSetOf())

private fun isPermanentSyncError(error: Throwable, seen: MutableSet<Throwable>): Boolean {
  var current: Throwable? = error
  while (current != null && seen.add(current)) {
    if (current is TypieError && current.code == "invalid_changeset_payload") return true
    if (current is ApolloHttpException && current.statusCode in 400..499) return true
    if (current is SubscriptionOperationException) return true
    for (suppressed in current.suppressedExceptions) {
      if (isPermanentSyncError(suppressed, seen)) return true
    }
    current = current.cause
  }
  return false
}

class GraphQlSyncTransport(
  private val documentId: String,
  private val clientId: String,
) : SyncTransport {
  override suspend fun push(changesets: ByteArray): PushResult {
    val response =
      Apollo.executeMutation(
        DocumentSync_PushDocumentChangesets_Mutation(
          input =
            PushDocumentChangesetsInput(
              documentId = documentId,
              clientId = clientId,
              changesets = changesets,
            )
        )
      )
    return PushResult(
      heads = response.pushDocumentChangesets.heads,
      durableHeads = response.pushDocumentChangesets.durableHeads,
    )
  }

  override suspend fun pull(sinceSeq: String?): PullResult {
    val response =
      Apollo.executeMutation(
        DocumentSync_PullDocumentChangesets_Mutation(
          input =
            PullDocumentChangesetsInput(
              documentId = documentId,
              sinceSeq = Optional.presentIfNotNull(sinceSeq),
            )
        )
      )
    val payload = response.pullDocumentChangesets
    return PullResult(
      changesets = payload.changesets,
      seq = payload.seq,
      heads = payload.heads,
      durableHeads = payload.durableHeads,
      needsReload = payload.needsReload,
    )
  }

  override fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent> =
    Apollo.subscription(
        DocumentSync_ChangesetsUpdated_Subscription(
          documentId = documentId,
          clientId = clientId,
          sinceSeq = Optional.presentIfNotNull(sinceSeq),
        )
      )
      .retryOnError(false)
      .toFlow()
      .map { response ->
        response.exception?.let { throw it }
        val graphError = response.errors?.firstOrNull()
        if (graphError != null) {
          val type = graphError.extensions?.get("type") as? String
          if (type == "TypieError") {
            val code = graphError.extensions?.get("code") as? String
            throw TypieError(code = code ?: "unknown", message = graphError.message)
          }
          throw Exception(graphError.message)
        }
        val event = response.dataOrThrow().documentChangesetsUpdated
        RemoteChangesetEvent(
          changesets = event.changesets,
          seq = event.seq,
          heads = event.heads,
          durableHeads = event.durableHeads,
        )
      }
}
