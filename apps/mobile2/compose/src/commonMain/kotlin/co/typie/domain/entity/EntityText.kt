package co.typie.domain.entity

import co.typie.ext.comma

const val UNTITLED_DOCUMENT_TEXT = "(제목 없음)"
const val UNNAMED_FOLDER_TEXT = "(이름 없음)"
const val EMPTY_ENTITY_EXCERPT_TEXT = "(내용 없음)"
const val EMPTY_SPACE_TEXT = "비어 있는 스페이스"
const val EMPTY_FOLDER_TEXT = "빈 폴더"

fun formatDocumentTitle(title: String, emptyText: String = UNTITLED_DOCUMENT_TEXT): String {
  return title.ifBlank { emptyText }
}

fun formatFolderName(name: String, emptyText: String = UNNAMED_FOLDER_TEXT): String {
  return name.ifBlank { emptyText }
}

fun formatEntityExcerpt(text: String, emptyText: String = EMPTY_ENTITY_EXCERPT_TEXT): String {
  return text.ifEmpty { emptyText }
}

fun formatSpaceSummary(folderCount: Int, documentCount: Int): String =
  formatEntitySummary(folderCount, documentCount, emptyText = EMPTY_SPACE_TEXT)

fun formatFolderSummary(folderCount: Int, documentCount: Int): String =
  formatEntitySummary(folderCount, documentCount, emptyText = EMPTY_FOLDER_TEXT)

fun formatFolderMetadataSummary(folderCount: Int, documentCount: Int, characterCount: Int): String {
  val parts = buildList {
    if (folderCount > 0) add("폴더 ${folderCount.comma}개")
    if (documentCount > 0) add("문서 ${documentCount.comma}개")
    add("총 ${characterCount.comma}자")
  }

  return parts.joinToString(" · ")
}

fun formatFolderRowSummary(folderCount: Int, documentCount: Int): String {
  if (folderCount == 0 && documentCount == 0) {
    return EMPTY_FOLDER_TEXT
  }

  if (folderCount == 0) {
    return "문서 ${documentCount.comma}개"
  }

  return "폴더 ${folderCount.comma}개 · 문서 ${documentCount.comma}개"
}

private fun formatEntitySummary(folderCount: Int, documentCount: Int, emptyText: String): String {
  val parts = buildList {
    if (folderCount > 0) add("폴더 ${folderCount.comma}개")
    if (documentCount > 0) add("문서 ${documentCount.comma}개")
  }

  return if (parts.isEmpty()) emptyText else parts.joinToString(" · ")
}
