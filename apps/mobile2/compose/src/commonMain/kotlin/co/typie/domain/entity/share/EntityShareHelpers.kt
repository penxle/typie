package co.typie.domain.entity

internal data class ShareThumbnailResult(val id: String, val url: String)

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
