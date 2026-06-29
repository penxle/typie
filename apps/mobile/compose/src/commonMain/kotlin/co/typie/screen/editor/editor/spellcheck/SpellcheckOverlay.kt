package co.typie.screen.editor.editor.spellcheck

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelHiddenScale
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityEnterMillis
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityExitMillis
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryGap
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.component.bleedingScrollFog
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.absoluteValue
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun SpellcheckOverlay(
  session: EditorSpellcheckSession,
  visibleArea: EditorVisibleArea,
  modifier: Modifier = Modifier,
) {
  val model = session.model ?: return
  val results = model.results
  val loading = model.check.loading
  val checked = model.check.data != null
  var displayedResults by remember { mutableStateOf<List<SpellcheckResult>>(emptyList()) }
  val resultIds = results.mapTo(mutableSetOf()) { it.id }
  val displayedResultIds = displayedResults.mapTo(mutableSetOf()) { it.id }
  val exitingResultIds = displayedResultIds - resultIds
  val pagerVisible =
    session.active && checked && (results.isNotEmpty() || displayedResults.isNotEmpty())
  val pagerTransition = remember { MutableTransitionState(false) }
  pagerTransition.targetState = pagerVisible

  LaunchedEffect(results) {
    if (exitingResultIds.isNotEmpty()) {
      delay(SpellcheckOverlayAnimationMillis.toLong())
      displayedResults = results
    } else if (results.isNotEmpty()) {
      displayedResults = results
    }
  }
  LaunchedEffect(pagerTransition.currentState, pagerTransition.targetState) {
    if (!pagerTransition.currentState && !pagerTransition.targetState) {
      displayedResults = emptyList()
      session.updateOverlayMetrics { copy(bottomOcclusion = 0f) }
    }
  }

  Box(modifier = modifier.fillMaxSize()) {
    val capsuleTopPadding =
      (visibleArea.visibleViewportTop.dp - SpellcheckCapsuleTopOverlap).coerceAtLeast(0.dp)
    AnimatedVisibility(
      visible = session.active,
      enter = spellcheckEnter(TransformOrigin(0.5f, 0f)),
      exit = spellcheckExit(TransformOrigin(0.5f, 0f)),
      modifier = Modifier.align(Alignment.TopCenter).padding(top = capsuleTopPadding),
    ) {
      SpellcheckCapsule(
        loading = loading,
        count = if (checked) results.size else null,
        onRerun = session.rerun,
        onClose = session.close,
        onHeightChanged = { height ->
          session.updateOverlayMetrics {
            copy(topOcclusion = (height - SpellcheckCapsuleTopOverlap.value).coerceAtLeast(0f))
          }
        },
      )
    }

    AnimatedVisibility(
      visibleState = pagerTransition,
      enter = spellcheckEnter(TransformOrigin(0.5f, 1f)),
      exit = spellcheckExit(TransformOrigin(0.5f, 1f)),
      modifier = Modifier.fillMaxSize(),
    ) {
      val pagerResults =
        if (exitingResultIds.isNotEmpty()) {
          displayedResults
        } else {
          results.ifEmpty { displayedResults }
        }
      if (pagerResults.isNotEmpty()) {
        SpellcheckResultPager(
          session = session,
          results = pagerResults,
          exitingResultIds = exitingResultIds,
          visibleArea = visibleArea,
          expanded = model.expanded,
          interactive = results.isNotEmpty() && exitingResultIds.isEmpty(),
          modifier = Modifier.fillMaxSize(),
        )
      }
    }
  }
}

@Composable
private fun SpellcheckCapsule(
  loading: Boolean,
  count: Int?,
  onRerun: () -> Unit,
  onClose: () -> Unit,
  onHeightChanged: (Float) -> Unit,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  var displayedCount by remember { mutableStateOf(count) }

  LaunchedEffect(count) {
    if (count != null) {
      displayedCount = count
    }
  }
  val visibleCount = count ?: displayedCount

  Row(
    modifier =
      modifier
        .animateContentSize(animationSpec = spellcheckTween())
        .onSizeChanged { size -> onHeightChanged(with(density) { size.height.toDp().value }) }
        .clip(AppShapes.rounded(AppShapes.full))
        .background(AppTheme.colors.surfaceDefault)
        .border(1.dp, AppTheme.colors.borderDefault, AppShapes.rounded(AppShapes.full))
        .padding(start = 14.dp, top = 8.dp, end = 6.dp, bottom = 8.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(8.dp),
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
        fadeIn(animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis)) +
          expandHorizontally(
            animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis),
            expandFrom = Alignment.Start,
          ),
      exit =
        fadeOut(animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis)) +
          shrinkHorizontally(
            animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis),
            shrinkTowards = Alignment.Start,
          ),
    ) {
      visibleCount?.let { SpellcheckCountBadge(it) }
    }
    if (loading) {
      Box(modifier = Modifier.size(28.dp), contentAlignment = Alignment.Center) {
        Spinner(color = AppTheme.colors.textMuted, size = 16.dp)
      }
    } else {
      SpellcheckIconButton(icon = Lucide.RefreshCw, contentDescription = "다시 검사", onClick = onRerun)
    }
    SpellcheckIconButton(icon = Lucide.X, contentDescription = "맞춤법 검사 닫기", onClick = onClose)
  }
}

@Composable
private fun SpellcheckCountBadge(count: Int) {
  Box(
    modifier =
      Modifier.animateContentSize(animationSpec = spellcheckTween())
        .clip(AppShapes.rounded(AppShapes.full))
        .background(AppTheme.colors.dangerSubtle)
        .padding(horizontal = 7.dp, vertical = 2.dp),
    contentAlignment = Alignment.Center,
  ) {
    AnimatedContent(
      targetState = count,
      transitionSpec = {
        (fadeIn(animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis)) togetherWith
            fadeOut(animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis)))
          .using(
            SizeTransform(clip = false) { _, _ -> tween(ToolbarBottomPanelVisibilityEnterMillis) }
          )
      },
      label = "SpellcheckCountBadge",
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
private fun SpellcheckIconButton(
  icon: IconData,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  Box(
    modifier = modifier.size(28.dp).clip(AppShapes.circle).clickable(onClick = onClick),
    contentAlignment = Alignment.Center,
  ) {
    Icon(
      icon = icon,
      contentDescription = contentDescription,
      modifier = Modifier.size(16.dp),
      tint = AppTheme.colors.textMuted,
    )
  }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun SpellcheckResultPager(
  session: EditorSpellcheckSession,
  results: List<SpellcheckResult>,
  exitingResultIds: Set<String>,
  visibleArea: EditorVisibleArea,
  expanded: Boolean,
  interactive: Boolean,
  modifier: Modifier = Modifier,
) {
  val pagerState = rememberPagerState(pageCount = { results.size })
  val currentCardId = session.model?.currentCardId
  val activeRangeId = session.model?.activeRangeId
  var userScrollPendingActivation by remember { mutableStateOf(false) }
  var programmaticScrollInProgress by remember { mutableStateOf(false) }
  val inactive = !expanded && activeRangeId == null
  val compactOverlayHeight = spellcheckCompactOverlayHeight(activeRange = !inactive)
  val bottomPadding = visibleArea.bottomOcclusion.dp + SpellcheckOverlayBottomGap
  val topOcclusion =
    if (session.occlusion.top > 0f) {
      session.occlusion.top.dp
    } else {
      EstimatedCapsuleHeight
    }
  val expandedTopReserve = visibleArea.visibleViewportTop.dp + topOcclusion + ExpandedTopGap

  SideEffect {
    if (results.isNotEmpty()) {
      session.updateOverlayMetrics { copy(bottomOcclusion = compactOverlayHeight.value) }
    }
  }
  LaunchedEffect(results.map { it.id }, currentCardId, interactive, exitingResultIds) {
    if (!interactive && exitingResultIds.isEmpty()) return@LaunchedEffect
    val page = results.indexOfFirst { it.id == currentCardId }
    if (page >= 0 && page != pagerState.currentPage) {
      userScrollPendingActivation = false
      programmaticScrollInProgress = true
      try {
        pagerState.animateScrollToPage(page, animationSpec = spellcheckTween())
      } finally {
        programmaticScrollInProgress = false
      }
    }
  }
  LaunchedEffect(pagerState, results, interactive) {
    if (!interactive) return@LaunchedEffect
    snapshotFlow {
        Triple(
          pagerState.isScrollInProgress,
          pagerState.currentPage,
          pagerState.currentPageOffsetFraction,
        )
      }
      .collectLatest { (scrolling, page, pageOffset) ->
        if (scrolling) {
          if (!programmaticScrollInProgress) {
            userScrollPendingActivation = true
          }
          return@collectLatest
        }
        if (!userScrollPendingActivation) return@collectLatest
        if (pageOffset.absoluteValue > PagerSettledOffsetTolerance) return@collectLatest
        userScrollPendingActivation = false
        val id = results.getOrNull(page)?.id ?: return@collectLatest
        if (id != session.model?.activeRangeId) {
          session.showCurrentResult(id)
          session.activateResult(id)
        }
      }
  }

  BoxWithConstraints(
    modifier = modifier.padding(bottom = bottomPadding),
    contentAlignment = Alignment.BottomCenter,
  ) {
    val expandedHeight = (maxHeight - expandedTopReserve).coerceAtLeast(0.dp)
    val pagerHeight by
      animateDpAsState(
        targetValue = if (expanded) expandedHeight else CompactCardHeight,
        animationSpec = spellcheckTween(),
        label = "SpellcheckPagerHeight",
      )
    val inactiveOffset by
      animateDpAsState(
        targetValue = if (inactive) InactiveCardOffset else 0.dp,
        animationSpec = spellcheckTween(),
        label = "SpellcheckInactiveCardOffset",
      )
    val showExpandedContent =
      expanded &&
        (expandedHeight - pagerHeight).value.absoluteValue <= ExpandedContentSettleTolerance

    HorizontalPager(
      state = pagerState,
      contentPadding = PaddingValues(horizontal = CompactPagerHorizontalPadding),
      pageSpacing = 12.dp,
      key = { page -> results[page].id },
      verticalAlignment = Alignment.Bottom,
      modifier = Modifier.fillMaxWidth().offset(y = inactiveOffset).height(pagerHeight),
    ) { page ->
      AnimatedContent(
        targetState = results[page],
        transitionSpec = {
          (spellcheckCardEnter() togetherWith spellcheckCardExit()).using(
            SizeTransform(clip = false) { _, _ -> tween(0) }
          )
        },
        contentKey = { result -> result.id },
        label = "SpellcheckResultCardContent",
        modifier = Modifier.fillMaxWidth().height(pagerHeight),
      ) { result ->
        val active = result.id == session.model?.activeRangeId
        SpellcheckResultCard(
          result = result,
          active = active,
          expanded = expanded,
          showExpandedContent = showExpandedContent,
          sameContextVisible = results.count { it.context == result.context } > 1,
          exiting = result.id in exitingResultIds,
          onClick = {
            if (interactive) {
              when {
                expanded -> session.setExpanded(false)
                active -> session.setExpanded(true)
                else -> session.activateResult(result.id)
              }
            }
          },
          onSuggestion = { replacement ->
            if (interactive) session.applySuggestion(result.id, replacement)
          },
          onDirectEdit = { if (interactive) session.directEdit(result.id) },
          onIgnoreSame = { if (interactive) session.ignoreSame(result.id) },
          onIgnore = { if (interactive) session.ignore(result.id) },
          modifier = Modifier.fillMaxSize(),
        )
      }
    }
  }
}

@Composable
private fun SpellcheckResultCard(
  result: SpellcheckResult,
  active: Boolean,
  expanded: Boolean,
  showExpandedContent: Boolean,
  sameContextVisible: Boolean,
  exiting: Boolean,
  onClick: () -> Unit,
  onSuggestion: (String) -> Unit,
  onDirectEdit: () -> Unit,
  onIgnoreSame: () -> Unit,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val shape = RoundedCornerShape(12.dp)
  val background = AppTheme.colors.surfaceDefault
  val borderColor by
    animateColorAsState(
      targetValue = if (active) AppTheme.colors.danger else AppTheme.colors.borderDefault,
      animationSpec = spellcheckTween(),
      label = "SpellcheckCardBorderColor",
    )

  AnimatedVisibility(
    visible = !exiting,
    enter = spellcheckCardEnter(),
    exit = spellcheckCardExit(),
    modifier = modifier,
  ) {
    Box(
      modifier =
        Modifier.fillMaxSize()
          .clip(shape)
          .background(background, shape)
          .border(1.dp, borderColor, shape)
          .clickable(onClick = onClick)
    ) {
      if (showExpandedContent) {
        ExpandedSpellcheckResultCardContent(
          result = result,
          sameContextVisible = sameContextVisible,
          background = background,
          onSuggestion = onSuggestion,
          onDirectEdit = onDirectEdit,
          onIgnoreSame = onIgnoreSame,
          onIgnore = onIgnore,
          modifier = Modifier.fillMaxSize(),
        )
      } else {
        CompactSpellcheckResultCardContent(
          result = result,
          sameContextVisible = sameContextVisible,
          background = background,
          onSuggestion = onSuggestion,
          onDirectEdit = onDirectEdit,
          onIgnoreSame = onIgnoreSame,
          onIgnore = onIgnore,
          modifier = Modifier.fillMaxSize().padding(SpellcheckCardPadding),
        )
      }
    }
  }
}

@Composable
private fun ExpandedSpellcheckResultCardContent(
  result: SpellcheckResult,
  sameContextVisible: Boolean,
  background: Color,
  onSuggestion: (String) -> Unit,
  onDirectEdit: () -> Unit,
  onIgnoreSame: () -> Unit,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val scrollState = rememberScrollState()

  SubcomposeLayout(modifier = modifier) { constraints ->
    val width = constraints.maxWidth
    val height = constraints.maxHeight
    val cardPaddingPx = SpellcheckCardPadding.roundToPx()
    val minContentHeightPx = (height - cardPaddingPx * 2).coerceAtLeast(0)
    val viewportConstraints =
      Constraints(minWidth = width, maxWidth = width, minHeight = height, maxHeight = height)
    val placeables =
      subcompose(ExpandedSpellcheckCardSlot.Viewport) {
          Box(modifier = Modifier.fillMaxSize().verticalScroll(scrollState)) {
            ExpandedSpellcheckResultCardLayout(
              result = result,
              sameContextVisible = sameContextVisible,
              background = background,
              minContentHeightPx = minContentHeightPx,
              onSuggestion = onSuggestion,
              onDirectEdit = onDirectEdit,
              onIgnoreSame = onIgnoreSame,
              onIgnore = onIgnore,
              modifier = Modifier.fillMaxWidth().padding(SpellcheckCardPadding),
            )
          }
        }
        .map { it.measure(viewportConstraints) }

    layout(width = width, height = height) {
      placeables.forEach { placeable -> placeable.placeRelative(0, 0) }
    }
  }
}

private enum class ExpandedSpellcheckCardSlot {
  Viewport,
  Body,
  Corrections,
  Actions,
}

@Composable
private fun ExpandedSpellcheckResultCardLayout(
  result: SpellcheckResult,
  sameContextVisible: Boolean,
  background: Color,
  minContentHeightPx: Int,
  onSuggestion: (String) -> Unit,
  onDirectEdit: () -> Unit,
  onIgnoreSame: () -> Unit,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  SubcomposeLayout(modifier = modifier) { constraints ->
    val contentWidth = constraints.maxWidth
    val rowConstraints =
      Constraints(
        minWidth = contentWidth,
        maxWidth = contentWidth,
        minHeight = 0,
        maxHeight = Constraints.Infinity,
      )

    val correctionsPlaceables =
      if (result.corrections.isNotEmpty()) {
        subcompose(ExpandedSpellcheckCardSlot.Corrections) {
            SpellcheckActionRow(color = background, modifier = Modifier.fillMaxWidth()) {
              result.corrections.forEach { correction ->
                SpellcheckActionChip(
                  text = correction,
                  danger = true,
                  onClick = { onSuggestion(correction) },
                )
              }
            }
          }
          .map { it.measure(rowConstraints) }
      } else {
        emptyList()
      }

    val actionsPlaceables =
      subcompose(ExpandedSpellcheckCardSlot.Actions) {
          SpellcheckActionRow(color = background, modifier = Modifier.fillMaxWidth()) {
            SpellcheckActionChip(text = "직접 수정", onClick = onDirectEdit)
            if (sameContextVisible) {
              SpellcheckActionChip(text = "같은 단어 모두 무시", onClick = onIgnoreSame)
            }
            SpellcheckActionChip(text = "무시", onClick = onIgnore)
          }
        }
        .map { it.measure(rowConstraints) }

    val correctionsHeight = correctionsPlaceables.maxOfOrNull { it.height } ?: 0
    val actionsHeight = actionsPlaceables.maxOfOrNull { it.height } ?: 0
    val gapPx = SpellcheckCardGap.roundToPx()
    val gapCount = if (result.corrections.isNotEmpty()) 2 else 1
    val minBodyHeight =
      (minContentHeightPx - correctionsHeight - actionsHeight - gapPx * gapCount).coerceAtLeast(0)

    val bodyPlaceables =
      subcompose(ExpandedSpellcheckCardSlot.Body) {
          ExpandedSpellcheckResultBody(result = result, modifier = Modifier.fillMaxWidth())
        }
        .map { it.measure(rowConstraints.copy(minHeight = minBodyHeight)) }
    val bodyHeight = bodyPlaceables.maxOfOrNull { it.height } ?: 0
    val contentHeight =
      (bodyHeight + correctionsHeight + actionsHeight + gapPx * gapCount).coerceAtLeast(
        minContentHeightPx
      )

    layout(width = contentWidth, height = contentHeight) {
      var y = 0
      bodyPlaceables.forEach { placeable -> placeable.placeRelative(0, y) }
      y += bodyHeight + gapPx

      if (correctionsPlaceables.isNotEmpty()) {
        correctionsPlaceables.forEach { placeable -> placeable.placeRelative(0, y) }
        y += correctionsHeight + gapPx
      }

      actionsPlaceables.forEach { placeable -> placeable.placeRelative(0, y) }
    }
  }
}

@Composable
private fun ExpandedSpellcheckResultBody(result: SpellcheckResult, modifier: Modifier = Modifier) {
  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(SpellcheckCardGap)) {
    Text(
      text = result.context,
      style = AppTheme.typography.title.copy(fontWeight = FontWeight.SemiBold),
      color = AppTheme.colors.danger,
      overflow = TextOverflow.Clip,
    )
    Text(
      text = result.explanation,
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
      modifier = Modifier.fillMaxWidth(),
      overflow = TextOverflow.Clip,
    )
  }
}

@Composable
private fun CompactSpellcheckResultCardContent(
  result: SpellcheckResult,
  sameContextVisible: Boolean,
  background: Color,
  onSuggestion: (String) -> Unit,
  onDirectEdit: () -> Unit,
  onIgnoreSame: () -> Unit,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(SpellcheckCardGap)) {
    Column(
      modifier = Modifier.weight(1f).fillMaxWidth(),
      verticalArrangement = Arrangement.spacedBy(SpellcheckCardGap),
    ) {
      Text(
        text = result.context,
        style = AppTheme.typography.title.copy(fontWeight = FontWeight.SemiBold),
        color = AppTheme.colors.danger,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
      var descriptionClamped by remember(result.id, result.explanation) { mutableStateOf(false) }
      val descriptionStyle = AppTheme.typography.caption.copy(color = AppTheme.colors.textMuted)
      Column(
        modifier = Modifier.weight(1f).fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(2.dp),
      ) {
        BasicText(
          text = result.explanation,
          modifier = Modifier.fillMaxWidth(),
          style = descriptionStyle,
          maxLines = CompactDescriptionMaxLines,
          overflow = TextOverflow.Ellipsis,
          onTextLayout = { layout ->
            val lastLineIndex = (layout.lineCount - 1).coerceAtLeast(0)
            descriptionClamped = layout.hasVisualOverflow || layout.isLineEllipsized(lastLineIndex)
          },
        )
        if (descriptionClamped) {
          Text(
            text = "더 보기",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.danger,
            maxLines = 1,
            overflow = TextOverflow.Clip,
          )
        }
      }
    }
    if (result.corrections.isNotEmpty()) {
      SpellcheckActionRow(color = background, modifier = Modifier.fillMaxWidth()) {
        result.corrections.forEach { correction ->
          SpellcheckActionChip(
            text = correction,
            danger = true,
            onClick = { onSuggestion(correction) },
          )
        }
      }
    }
    SpellcheckActionRow(color = background, modifier = Modifier.fillMaxWidth()) {
      SpellcheckActionChip(text = "직접 수정", onClick = onDirectEdit)
      if (sameContextVisible) {
        SpellcheckActionChip(text = "같은 단어 모두 무시", onClick = onIgnoreSame)
      }
      SpellcheckActionChip(text = "무시", onClick = onIgnore)
    }
  }
}

@Composable
private fun SpellcheckActionRow(
  color: Color,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  val scrollState = rememberScrollState()
  Box(
    modifier =
      modifier.bleedingScrollFog(
        padding = PaddingValues(horizontal = ActionRowFogWidth),
        color = color,
      )
  ) {
    Row(
      modifier = Modifier.horizontalScroll(scrollState).padding(horizontal = ActionRowFogWidth),
      horizontalArrangement = Arrangement.spacedBy(6.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      content()
    }
  }
}

@Composable
private fun SpellcheckActionChip(text: String, danger: Boolean = false, onClick: () -> Unit) {
  val background = if (danger) AppTheme.colors.dangerSubtle else AppTheme.colors.surfaceInset
  val foreground = if (danger) AppTheme.colors.textOnDangerSubtle else AppTheme.colors.textDefault
  Box(
    modifier =
      Modifier.clip(AppShapes.rounded(AppShapes.full))
        .background(background)
        .clickable(onClick = onClick)
        .padding(horizontal = 10.dp, vertical = 6.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text = text, style = AppTheme.typography.action, color = foreground, maxLines = 1)
  }
}

internal const val SpellcheckOverlayAnimationMillis = 180
private val CompactPagerHorizontalPadding = 24.dp
private val ExpandedTopGap = 8.dp
private val SpellcheckCapsuleTopOverlap = TopBarDefaults.ContentTopSpacing
private val SpellcheckOverlayBottomGap = ToolbarSecondaryGap
private val SpellcheckCardPadding = 14.dp
private val SpellcheckCardGap = 8.dp
private val ActionRowFogWidth = 16.dp
private val CompactCardHeight = 200.dp
private val InactiveCardOffset = 92.dp
private val EstimatedCapsuleHeight = 44.dp
private const val CompactDescriptionMaxLines = 2
private const val ExpandedContentSettleTolerance = 0.5f
private const val PagerSettledOffsetTolerance = 0.001f

internal fun spellcheckCompactOverlayHeight(activeRange: Boolean) =
  CompactCardHeight + SpellcheckOverlayBottomGap - if (activeRange) 0.dp else InactiveCardOffset

private fun <T> spellcheckTween() =
  tween<T>(durationMillis = SpellcheckOverlayAnimationMillis, easing = LinearOutSlowInEasing)

private fun spellcheckEnter(transformOrigin: TransformOrigin) =
  fadeIn(animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis)) +
    scaleIn(
      animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis),
      initialScale = ToolbarBottomPanelHiddenScale,
      transformOrigin = transformOrigin,
    )

private fun spellcheckExit(transformOrigin: TransformOrigin) =
  fadeOut(animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis)) +
    scaleOut(
      animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis),
      targetScale = ToolbarBottomPanelHiddenScale,
      transformOrigin = transformOrigin,
    )

private fun spellcheckCardEnter() = spellcheckEnter(TransformOrigin(0.5f, 1f))

private fun spellcheckCardExit() = spellcheckExit(TransformOrigin(0.5f, 1f))
