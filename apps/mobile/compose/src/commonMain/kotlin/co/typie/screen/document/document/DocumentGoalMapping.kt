package co.typie.screen.document.document

import co.typie.domain.goal.GoalSourceCandidate
import co.typie.domain.goal.toEntityGoalData
import co.typie.graphql.DocumentScreen_Query

internal fun DocumentScreen_Query.Ancestor.toGoalSourceCandidate(): GoalSourceCandidate =
  GoalSourceCandidate(
    entityId = id,
    goal = goal?.entityGoalFields_goal?.toEntityGoalData(),
    folderCharacterCount = node.onFolder?.characterCount?.toLong(),
  )

internal fun List<DocumentScreen_Query.Ancestor>.toGoalSourceCandidates():
  List<GoalSourceCandidate> = map { it.toGoalSourceCandidate() }
