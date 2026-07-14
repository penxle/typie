package co.typie.screen.editor.editor.spellcheck

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
internal fun SpellcheckTopBarLeading(session: EditorSpellcheckSession) {
  TopBarButton(
    icon = Lucide.X,
    onClick = { session.close() },
    modifier =
      Modifier.semantics {
        contentDescription = "맞춤법 검사 닫기"
        role = Role.Button
      },
  )
}

@Composable
internal fun SpellcheckTopBarCenter(session: EditorSpellcheckSession) {
  val model = session.model ?: return
  val count = if (model.ready) model.results.size else null
  var displayedCount by remember { mutableStateOf(count) }
  val shape = AppShapes.rounded(AppShapes.full)

  LaunchedEffect(count) {
    if (count != null) {
      displayedCount = count
    }
  }
  val visibleCount = count ?: displayedCount

  Row(
    modifier =
      Modifier.fillMaxWidth()
        .height(TopBarDefaults.TitleHeight)
        .shadow(AppTheme.shadows.sm, shape)
        .clip(shape)
        .background(TopBarDefaults.controlBackgroundColor(), shape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), shape)
        .padding(horizontal = 14.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.Center,
  ) {
    Text(
      text = "맞춤법 검사",
      style = AppTheme.typography.action,
      color = AppTheme.colors.textDefault,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    AnimatedVisibility(
      visible = count != null,
      enter =
        fadeIn(animationSpec = tween(SpellcheckOverlayAnimationMillis)) +
          expandHorizontally(
            animationSpec = tween(SpellcheckOverlayAnimationMillis),
            expandFrom = Alignment.Start,
          ),
      exit =
        fadeOut(animationSpec = tween(SpellcheckOverlayAnimationMillis)) +
          shrinkHorizontally(
            animationSpec = tween(SpellcheckOverlayAnimationMillis),
            shrinkTowards = Alignment.Start,
          ),
    ) {
      visibleCount?.let {
        Box(modifier = Modifier.padding(start = 8.dp)) { SpellcheckTopBarCountBadge(it) }
      }
    }
  }
}

@Composable
internal fun SpellcheckTopBarTrailing(session: EditorSpellcheckSession) {
  val loading = session.model?.loading == true

  if (loading) {
    SpellcheckTopBarSpinner()
  } else {
    TopBarButton(
      icon = Lucide.RefreshCw,
      onClick = { session.rerun() },
      modifier =
        Modifier.semantics {
          contentDescription = "다시 검사"
          role = Role.Button
        },
    )
  }
}

@Composable
private fun SpellcheckTopBarCountBadge(count: Int) {
  Box(
    modifier =
      Modifier.clip(AppShapes.rounded(AppShapes.full))
        .background(AppTheme.colors.dangerSubtle)
        .padding(horizontal = 7.dp, vertical = 2.dp),
    contentAlignment = Alignment.Center,
  ) {
    AnimatedContent(
      targetState = count,
      transitionSpec = {
        (fadeIn(animationSpec = tween(SpellcheckOverlayAnimationMillis)) togetherWith
            fadeOut(animationSpec = tween(SpellcheckOverlayAnimationMillis)))
          .using(SizeTransform(clip = false) { _, _ -> tween(SpellcheckOverlayAnimationMillis) })
      },
      label = "SpellcheckTopBarCountBadge",
    ) { value ->
      Text(
        text = value.toString(),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textOnDangerSubtle,
        maxLines = 1,
      )
    }
  }
}

@Composable
private fun SpellcheckTopBarSpinner() {
  Box(
    modifier =
      Modifier.size(TopBarDefaults.ButtonSize)
        .shadow(AppTheme.shadows.sm, TopBarDefaults.ButtonShape)
        .background(TopBarDefaults.controlBackgroundColor(), TopBarDefaults.ButtonShape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), TopBarDefaults.ButtonShape)
        .semantics { contentDescription = "맞춤법 검사 중" },
    contentAlignment = Alignment.Center,
  ) {
    Spinner(color = AppTheme.colors.textMuted, size = 16.dp)
  }
}
