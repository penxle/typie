package co.typie.screen.goal.entity

import co.typie.datetime.toKstLocalDate
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entity.formatFolderName
import co.typie.domain.goal.CharacterCountPoint
import co.typie.graphql.EntityGoalScreen_Query

internal fun EntityGoalScreen_Query.CharacterCountHistory.toCharacterCountPoint():
  CharacterCountPoint =
  CharacterCountPoint(date = date.toKstLocalDate(), characterCount = characterCount.toLong())

internal fun List<EntityGoalScreen_Query.CharacterCountHistory>.toCharacterCountPoints():
  List<CharacterCountPoint> = map { it.toCharacterCountPoint() }.sortedBy { it.date }

internal fun EntityGoalScreen_Query.Node.targetName(): String {
  val document = onDocument
  if (document != null) {
    return formatDocumentTitle(document.title)
  }

  val folder = onFolder
  return folder?.let { formatFolderName(it.name) } ?: formatDocumentTitle("")
}

internal fun EntityGoalScreen_Query.Node.currentCharacterCount(): Long =
  (onDocument?.characterCount ?: onFolder?.characterCount ?: 0).toLong()
