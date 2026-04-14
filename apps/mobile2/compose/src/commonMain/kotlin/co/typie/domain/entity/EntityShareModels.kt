package co.typie.domain.entity

import co.typie.graphql.type.EntityType

internal enum class EntityShareKind {
  Folder,
  Document,
}

internal data class ShareThumbnailResult(val id: String, val url: String)

internal fun resolveEntityShareKind(types: List<EntityType>): EntityShareKind? {
  val distinctTypes = types.distinct()
  if (distinctTypes.size != 1) {
    return null
  }

  return when (distinctTypes.single()) {
    EntityType.FOLDER -> EntityShareKind.Folder
    EntityType.DOCUMENT -> EntityShareKind.Document
    else -> null
  }
}

internal fun resolveEntityShareTitle(kind: EntityShareKind, count: Int): String {
  val isSingle = count <= 1

  return when (kind) {
    EntityShareKind.Folder -> if (isSingle) "이 폴더 공유하기" else "폴더 ${count}개 공유하기"
    EntityShareKind.Document -> if (isSingle) "이 문서 공유하기" else "문서 ${count}개 공유하기"
  }
}

internal fun resolveEntityShareText(urls: List<String>): String? {
  val resolvedUrls = urls.map(String::trim).filter(String::isNotEmpty)
  if (resolvedUrls.isEmpty()) {
    return null
  }

  return resolvedUrls.joinToString("\n")
}

internal fun <T> hasMixedValues(values: List<T>): Boolean = values.distinct().size > 1

internal fun <T> resolveSharedValue(values: List<T>): T? {
  if (values.isEmpty() || hasMixedValues(values)) {
    return null
  }

  return values.first()
}
