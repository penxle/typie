package co.typie.domain.goal

import androidx.compose.runtime.Composable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import co.typie.ui.theme.AppTheme

val goalHeroNumberStyle: TextStyle
  @Composable
  get() =
    AppTheme.typography.display.copy(
      fontSize = GoalHeroNumberFontSize,
      lineHeight = GoalHeroNumberLineHeight,
      fontWeight = FontWeight.Bold,
      fontFeatureSettings = GOAL_HERO_NUMBER_TABULAR_FIGURES,
    )

private const val GOAL_HERO_NUMBER_TABULAR_FIGURES = "tnum"

private val GoalHeroNumberFontSize = 32.sp
private val GoalHeroNumberLineHeight = 40.sp
