package co.typie.domain.entity

import co.typie.graphql.fragment.EntityDetailsDocument_document
import co.typie.graphql.fragment.EntityDetailsFolder_folder
import co.typie.graphql.fragment.EntityDetails_entity
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityVisibility

val EntityDetails_entity.document: EntityDetailsDocument_document?
  get() = node.onDocument?.entityDetailsDocument_document

val EntityDetails_entity.folder: EntityDetailsFolder_folder?
  get() = node.onFolder?.entityDetailsFolder_folder

fun EntityDetails_entity.breadcrumbNames(): List<String> {
  return entityBreadcrumb_entity.breadcrumbSegments()
}

internal data class EntityVisibilityPresentation(val label: String, val isShared: Boolean)

internal fun entityVisibilityPresentation(
  entity: EntityDetails_entity?
): EntityVisibilityPresentation {
  return entityVisibilityPresentation(
    visibility = entity?.visibility,
    availability = entity?.availability,
  )
}

internal fun entityVisibilityPresentation(
  visibility: EntityVisibility?,
  availability: EntityAvailability?,
): EntityVisibilityPresentation {
  return when {
    visibility == EntityVisibility.PUBLIC ->
      EntityVisibilityPresentation(label = "공개", isShared = true)
    visibility == EntityVisibility.UNLISTED && availability == EntityAvailability.UNLISTED ->
      EntityVisibilityPresentation(label = "링크 조회/편집 가능", isShared = true)
    visibility == EntityVisibility.UNLISTED ->
      EntityVisibilityPresentation(label = "링크 조회 가능", isShared = true)
    availability == EntityAvailability.UNLISTED ->
      EntityVisibilityPresentation(label = "링크 편집 가능", isShared = true)
    else -> EntityVisibilityPresentation(label = "비공개", isShared = false)
  }
}
