package co.typie.domain.entitytransfer

import co.typie.domain.entity.document
import co.typie.domain.entity.folder
import co.typie.graphql.fragment.EntityDetails_entity
import co.typie.graphql.fragment.EntityRow_entity

const val EntityTransferMaxDepth = 100

sealed interface EntityTransferSource {
  val id: String
  val title: String
  val depth: Int

  val subtreeFolderDepth: Int

  fun canMoveToDepth(destinationDepth: Int): Boolean {
    val targetDepth = destinationDepth + 1
    return targetDepth + subtreeFolderDepth <= EntityTransferMaxDepth
  }

  data class Folder(
    override val id: String,
    override val title: String,
    override val depth: Int,
    val maxDescendantFoldersDepth: Int,
  ) : EntityTransferSource {
    override val subtreeFolderDepth: Int
      get() = maxDescendantFoldersDepth - depth + 1
  }

  data class Document(
    override val id: String,
    override val title: String,
    override val depth: Int,
  ) : EntityTransferSource {
    override val subtreeFolderDepth: Int = 0
  }
}

fun EntityDetails_entity.toTransferSource(): EntityTransferSource {
  return entityRow_entity.toTransferSource()
}

fun EntityRow_entity.toTransferSource(): EntityTransferSource {
  val document = document
  val folder = folder

  return when {
    document != null -> {
      EntityTransferSource.Document(id = id, title = document.title, depth = depth)
    }

    folder != null -> {
      EntityTransferSource.Folder(
        id = id,
        title = folder.name,
        depth = depth,
        maxDescendantFoldersDepth = folder.maxDescendantFoldersDepth,
      )
    }

    else -> error("Entity transfer source requires a document or folder entity: $id")
  }
}
