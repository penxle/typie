package co.typie.screen.editor.editor.aifeedback

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ext.clickable
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelHiddenScale
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityEnterMillis
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityExitMillis
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryGap
import co.typie.ui.component.Text
import co.typie.ui.component.bleedingScrollFog
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.absoluteValue
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun AiFeedbackOverlay(
  session: EditorAiFeedbackSession,
  visibleArea: EditorVisibleArea,
  modifier: Modifier = Modifier,
) {
  val model = session.model ?: return
  val results = model.results
  var displayedResults by remember { mutableStateOf<List<AiFeedbackResult>>(emptyList()) }
  val resultIds = results.mapTo(mutableSetOf()) { it.id }
  val displayedResultIds = displayedResults.mapTo(mutableSetOf()) { it.id }
  val exitingResultIds = displayedResultIds - resultIds
  val pagerVisible = session.active && (results.isNotEmpty() || displayedResults.isNotEmpty())
  val pagerTransition = remember { MutableTransitionState(false) }
  pagerTransition.targetState = pagerVisible

  LaunchedEffect(results) {
    if (exitingResultIds.isNotEmpty()) {
      delay(AiFeedbackOverlayAnimationMillis.toLong())
      displayedResults = results
    } else if (results.isNotEmpty()) {
      displayedResults = results
    }
  }
  LaunchedEffect(pagerTransition.currentState, pagerTransition.targetState) {
    if (!pagerTransition.currentState && !pagerTransition.targetState) {
      displayedResults = emptyList()
      session.setOverlayBottomOcclusion(0f)
    }
  }

  Box(modifier = modifier.fillMaxSize()) {
    AnimatedVisibility(
      visibleState = pagerTransition,
      enter = aiFeedbackEnter(TransformOrigin(0.5f, 1f)),
      exit = aiFeedbackExit(TransformOrigin(0.5f, 1f)),
      modifier = Modifier.fillMaxSize(),
    ) {
      val pagerResults =
        if (exitingResultIds.isNotEmpty()) {
          displayedResults
        } else {
          results.ifEmpty { displayedResults }
        }
      if (pagerResults.isNotEmpty()) {
        AiFeedbackResultPager(
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

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun AiFeedbackResultPager(
  session: EditorAiFeedbackSession,
  results: List<AiFeedbackResult>,
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
  val compactOverlayHeight = aiFeedbackCompactOverlayHeight(activeRange = !inactive)
  val bottomPadding = visibleArea.bottomOcclusion.dp + AiFeedbackOverlayBottomGap
  val expandedTopReserve = visibleArea.visibleViewportTop.dp + ExpandedTopGap

  SideEffect {
    if (results.isNotEmpty()) {
      session.setOverlayBottomOcclusion(compactOverlayHeight.value)
    }
  }
  LaunchedEffect(results.size, pagerState.currentPage) {
    val lastPage = results.lastIndex
    if (lastPage >= 0 && pagerState.currentPage > lastPage) {
      pagerState.scrollToPage(lastPage)
    }
  }
  LaunchedEffect(results.map { it.id }, currentCardId, interactive, exitingResultIds) {
    if (!interactive && exitingResultIds.isEmpty()) return@LaunchedEffect
    val page = results.indexOfFirst { it.id == currentCardId }
    if (page >= 0 && page != pagerState.currentPage) {
      userScrollPendingActivation = false
      programmaticScrollInProgress = true
      try {
        pagerState.animateScrollToPage(page, animationSpec = aiFeedbackTween())
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
        animationSpec = aiFeedbackTween(),
        label = "AiFeedbackPagerHeight",
      )
    val inactiveOffset by
      animateDpAsState(
        targetValue = if (inactive) InactiveCardOffset else 0.dp,
        animationSpec = aiFeedbackTween(),
        label = "AiFeedbackInactiveCardOffset",
      )
    val showExpandedContent =
      expanded &&
        (expandedHeight - pagerHeight).value.absoluteValue <= ExpandedContentSettleTolerance

    HorizontalPager(
      state = pagerState,
      contentPadding = PaddingValues(horizontal = CompactPagerHorizontalPadding),
      pageSpacing = 12.dp,
      key = { page -> results.getOrNull(page)?.id ?: "ai-feedback-page-$page" },
      verticalAlignment = Alignment.Bottom,
      modifier = Modifier.fillMaxWidth().offset(y = inactiveOffset).height(pagerHeight),
    ) { page ->
      val result = results.getOrNull(page) ?: return@HorizontalPager
      AnimatedContent(
        targetState = result,
        transitionSpec = {
          (aiFeedbackCardEnter() togetherWith aiFeedbackCardExit()).using(
            SizeTransform(clip = false) { _, _ -> tween(0) }
          )
        },
        contentKey = { result -> result.id },
        label = "AiFeedbackResultCardContent",
        modifier = Modifier.fillMaxWidth().height(pagerHeight),
      ) { result ->
        val active = result.id == session.model?.activeRangeId
        AiFeedbackResultCard(
          result = result,
          active = active,
          expanded = expanded,
          showExpandedContent = showExpandedContent,
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
          onIgnore = { if (interactive) session.ignore(result.id) },
          modifier = Modifier.fillMaxSize(),
        )
      }
    }
  }
}

@Composable
private fun AiFeedbackResultCard(
  result: AiFeedbackResult,
  active: Boolean,
  expanded: Boolean,
  showExpandedContent: Boolean,
  exiting: Boolean,
  onClick: () -> Unit,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val shape = RoundedCornerShape(12.dp)
  val background = AppTheme.colors.surfaceDefault
  val borderColor by
    animateColorAsState(
      targetValue = if (active) AppTheme.colors.palette.purple else AppTheme.colors.borderDefault,
      animationSpec = aiFeedbackTween(),
      label = "AiFeedbackCardBorderColor",
    )

  AnimatedVisibility(
    visible = !exiting,
    enter = aiFeedbackCardEnter(),
    exit = aiFeedbackCardExit(),
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
        ExpandedAiFeedbackResultCardContent(
          result = result,
          background = background,
          onIgnore = onIgnore,
          modifier = Modifier.fillMaxSize().padding(AiFeedbackCardPadding),
        )
      } else {
        CompactAiFeedbackResultCardContent(
          result = result,
          background = background,
          onIgnore = onIgnore,
          modifier = Modifier.fillMaxSize().padding(AiFeedbackCardPadding),
        )
      }
    }
  }
}

@Composable
private fun ExpandedAiFeedbackResultCardContent(
  result: AiFeedbackResult,
  background: Color,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val scrollState = rememberScrollState()
  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(AiFeedbackCardGap)) {
    Column(
      modifier = Modifier.weight(1f).fillMaxWidth().verticalScroll(scrollState),
      verticalArrangement = Arrangement.spacedBy(AiFeedbackCardGap),
    ) {
      AiFeedbackResultHeader(result = result)
      Text(
        text = result.feedback,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textMuted,
        modifier = Modifier.fillMaxWidth(),
        overflow = TextOverflow.Clip,
      )
    }
    AiFeedbackActionRow(color = background, modifier = Modifier.fillMaxWidth()) {
      AiFeedbackActionChip(text = "무시", onClick = onIgnore)
    }
  }
}

@Composable
private fun CompactAiFeedbackResultCardContent(
  result: AiFeedbackResult,
  background: Color,
  onIgnore: () -> Unit,
  modifier: Modifier = Modifier,
) {
  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(AiFeedbackCardGap)) {
    Column(
      modifier = Modifier.weight(1f).fillMaxWidth(),
      verticalArrangement = Arrangement.spacedBy(AiFeedbackCardGap),
    ) {
      AiFeedbackResultHeader(result = result)
      var feedbackClamped by remember(result.id, result.feedback) { mutableStateOf(false) }
      val feedbackStyle = AppTheme.typography.caption.copy(color = AppTheme.colors.textMuted)
      Column(
        modifier = Modifier.weight(1f).fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(2.dp),
      ) {
        BasicText(
          text = result.feedback,
          modifier = Modifier.fillMaxWidth(),
          style = feedbackStyle,
          maxLines = CompactFeedbackMaxLines,
          overflow = TextOverflow.Ellipsis,
          onTextLayout = { layout ->
            val lastLineIndex = (layout.lineCount - 1).coerceAtLeast(0)
            feedbackClamped = layout.hasVisualOverflow || layout.isLineEllipsized(lastLineIndex)
          },
        )
        if (feedbackClamped) {
          Text(
            text = "더 보기",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.palette.purple,
            maxLines = 1,
            overflow = TextOverflow.Clip,
          )
        }
      }
    }
    AiFeedbackActionRow(color = background, modifier = Modifier.fillMaxWidth()) {
      AiFeedbackActionChip(text = "무시", onClick = onIgnore)
    }
  }
}

@Composable
private fun AiFeedbackResultHeader(result: AiFeedbackResult, modifier: Modifier = Modifier) {
  Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
    result.category
      ?.takeIf { it.isNotBlank() }
      ?.let { category ->
        Box(
          modifier =
            Modifier.clip(AppShapes.rounded(AppShapes.full))
              .background(AppTheme.colors.palette.purple.copy(alpha = 0.14f))
              .padding(horizontal = 8.dp, vertical = 3.dp)
        ) {
          Text(
            text = category,
            style = AppTheme.typography.caption,
            color = AppTheme.colors.palette.purple,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
    Text(
      text = result.quotedTarget,
      style = AppTheme.typography.title.copy(fontWeight = FontWeight.SemiBold),
      color = AppTheme.colors.palette.purple,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun AiFeedbackActionRow(
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
private fun AiFeedbackActionChip(text: String, onClick: () -> Unit) {
  Box(
    modifier =
      Modifier.clip(AppShapes.rounded(AppShapes.full))
        .background(AppTheme.colors.surfaceInset)
        .clickable(onClick = onClick)
        .padding(horizontal = 10.dp, vertical = 6.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text = text, style = AppTheme.typography.action, color = AppTheme.colors.textDefault)
  }
}

private val AiFeedbackResult.quotedTarget: String
  get() {
    val start = startText.trim()
    val end = endText.trim()
    return if (end.isBlank() || end == start) {
      "\"$start\""
    } else {
      "\"$start\" ... \"$end\""
    }
  }

internal const val AiFeedbackOverlayAnimationMillis = 180
private val CompactPagerHorizontalPadding = 24.dp
private val ExpandedTopGap = 8.dp
private val AiFeedbackOverlayBottomGap = ToolbarSecondaryGap
private val AiFeedbackCardPadding = 14.dp
private val AiFeedbackCardGap = 8.dp
private val ActionRowFogWidth = 16.dp
private val CompactCardHeight = 200.dp
private val InactiveCardOffset = 92.dp
private const val CompactFeedbackMaxLines = 2
private const val ExpandedContentSettleTolerance = 0.5f
private const val PagerSettledOffsetTolerance = 0.001f

internal fun aiFeedbackCompactOverlayHeight(activeRange: Boolean) =
  CompactCardHeight + AiFeedbackOverlayBottomGap - if (activeRange) 0.dp else InactiveCardOffset

private fun <T> aiFeedbackTween() =
  tween<T>(durationMillis = AiFeedbackOverlayAnimationMillis, easing = LinearOutSlowInEasing)

private fun aiFeedbackEnter(transformOrigin: TransformOrigin) =
  fadeIn(animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis)) +
    scaleIn(
      animationSpec = tween(ToolbarBottomPanelVisibilityEnterMillis),
      initialScale = ToolbarBottomPanelHiddenScale,
      transformOrigin = transformOrigin,
    )

private fun aiFeedbackExit(transformOrigin: TransformOrigin) =
  fadeOut(animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis)) +
    scaleOut(
      animationSpec = tween(ToolbarBottomPanelVisibilityExitMillis),
      targetScale = ToolbarBottomPanelHiddenScale,
      transformOrigin = transformOrigin,
    )

private fun aiFeedbackCardEnter() = aiFeedbackEnter(TransformOrigin(0.5f, 1f))

private fun aiFeedbackCardExit() = aiFeedbackExit(TransformOrigin(0.5f, 1f))
