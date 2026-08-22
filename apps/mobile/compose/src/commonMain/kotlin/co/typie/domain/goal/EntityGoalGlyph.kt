package co.typie.domain.goal

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.datetime.toKstLocalDate
import co.typie.graphql.fragment.EntityGoalGlyph_entity
import co.typie.ui.component.ProgressRing
import kotlin.time.Clock

@Composable
fun EntityGoalGlyph(entity: EntityGoalGlyph_entity, modifier: Modifier = Modifier) {
  val goal = entity.goal?.entityGoalFields_goal ?: return
  val current =
    entity.node.onDocument?.characterCount ?: entity.node.onFolder?.characterCount ?: return
  val now = remember { Clock.System.now() }
  val glyph =
    remember(current, goal, now) {
      entityGoalGlyphState(
        current = current.toLong(),
        target = goal.targetCharacterCount.toLong(),
        createdDate = goal.createdAt.toKstLocalDate(),
        dueDate = goal.dueAt?.toKstLocalDate(),
        today = now.toKstLocalDate(),
        now = now,
      )
    }

  ProgressRing(
    progress = current.toFloat() / goal.targetCharacterCount.toFloat(),
    modifier = modifier,
    state = glyph.colorState,
    pie = glyph.pie,
    pieWarning = glyph.pieWarning,
    size = EntityGoalGlyphSize,
  )
}

private val EntityGoalGlyphSize = 16.dp
