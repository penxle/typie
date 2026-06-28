package co.typie.screen.editor.editor.subpane.comments

internal sealed interface CommentThreadLocation {
  data class Located(val excerpt: String) : CommentThreadLocation

  data object Missing : CommentThreadLocation
}

internal fun commentThreadLocation(excerpt: String?): CommentThreadLocation {
  val normalized = excerpt?.trim()?.replace(Regex("\\s+"), " ")
  return normalized?.takeIf { it.isNotEmpty() }?.let { CommentThreadLocation.Located(excerpt = it) }
    ?: CommentThreadLocation.Missing
}
