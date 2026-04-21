package co.typie.domain.entity

import co.typie.graphql.fragment.EntityParentMeta_folder
import co.typie.graphql.fragment.EntityRowDocument_document
import co.typie.graphql.fragment.EntityRowFolder_folder
import co.typie.graphql.fragment.EntityRowParent_entity
import co.typie.graphql.fragment.EntityRow_entity

val EntityRow_entity.document: EntityRowDocument_document?
  get() = node.onDocument?.entityRowDocument_document

val EntityRow_entity.folder: EntityRowFolder_folder?
  get() = node.onFolder?.entityRowFolder_folder

fun EntityRow_entity.isRowEntity(): Boolean {
  return document != null || folder != null
}

fun EntityRow_entity.isFolder(): Boolean {
  return folder != null
}

fun EntityRow_entity.displayTitle(): String {
  val document = document
  if (document != null) {
    return formatDocumentTitle(document.title)
  }

  val folder = folder
  return folder?.let { formatFolderName(it.name) } ?: formatDocumentTitle("")
}

fun EntityRow_entity.displayPreviewText(
  emptyDocumentText: String = "문서",
  emptyFolderText: String = "폴더",
): String? {
  val document = document
  if (document != null) {
    return document.excerpt.takeIf { it.isNotBlank() }
      ?: document.subtitle?.takeIf { it.isNotBlank() }
      ?: emptyDocumentText
  }

  return if (folder != null) emptyFolderText else emptyDocumentText
}

fun EntityRowParent_entity.parentFolderMeta(): EntityParentMeta_folder? {
  return parent?.node?.onFolder?.entityParentMeta_folder
}
