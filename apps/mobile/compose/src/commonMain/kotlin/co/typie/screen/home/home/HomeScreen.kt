package co.typie.screen.home.home

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.lerp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.EntityRow
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entity.formatEntityExcerpt
import co.typie.domain.entity.parentFolderMeta
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.comma
import co.typie.ext.navigationBarsPadding
import co.typie.ext.plus
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.ext.truncate
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.HomeScreen_ContinueWriting_document
import co.typie.graphql.fragment.HomeScreen_RecentDocumentRow_document
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.shell.LocalTabState
import co.typie.shell.MainBottomBarPillEntry
import co.typie.shell.MainBottomBarPillKey
import co.typie.shell.MainDrawerTrigger
import co.typie.shell.MainDrawerTriggerLeadingKey
import co.typie.shell.Tab
import co.typie.ui.component.Divider
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.BottomBarAction
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.skeleton.SkeletonDefaults
import co.typie.ui.state.rememberPagerState
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.AppTypography.title
import co.typie.ui.theme.PaperlogyFontFamily
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.HazeLogger.enabled
import kotlin.math.abs
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.time.Clock
import kotlin.time.Duration.Companion.hours
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private val ContinueWritingPinHeight: Dp = 56.dp
private val ContinueWritingPinGap: Dp = 12.dp
private val HomeSearchBarHeight: Dp = 48.dp
private const val ContinueWritingPinDragFadeStrength = 0.6f

@Composable
fun HomeScreen() {
  val model = viewModel { HomeViewModel() }

  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current

  ProvideTopBar(
    leadingKey = MainDrawerTriggerLeadingKey,
    leading = { MainDrawerTrigger() },
    trailing = null,
  )

  ProvideBottomBar(
    pillKey = MainBottomBarPillKey,
    pill = MainBottomBarPillEntry,
    action =
      BottomBarAction(
        icon = Lucide.Pencil,
        onClick = {
          if (model.isCreatingDocument) return@BottomBarAction
          model.createDocument().withDefaultExceptionHandler(toast).onOk {
            nav.navigate(Route.Editor(it))
          }
        },
      ),
  )

  val continueDocs =
    model.query.data.me.recentlyViewedEntities.mapNotNull {
      it.node.onDocument?.homeScreen_ContinueWriting_document
    }

  val recentDocs =
    model.query.data.me.recentlyViewedEntities.mapNotNull {
      it.node.onDocument?.homeScreen_RecentDocumentRow_document
    }

  val continueWritingDoc = model.continueWritingDocument

  val createDocument: suspend () -> Unit = {
    if (!model.isCreatingDocument) {
      model.createDocument().withDefaultExceptionHandler(toast).onOk {
        nav.navigate(Route.Editor(it))
      }
    }
  }

  Screen(
    loadable = model.query,
    contentPadding = PaddingValues.Zero,
    overlay =
      continueWritingDoc?.let { doc ->
        {
          Column(
            modifier =
              Modifier.align(Alignment.BottomCenter)
                .navigationBarsPadding()
                .padding(bottom = BottomBarDefaults.BarAreaHeight + ContinueWritingPinGap)
                .padding(horizontal = 16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
          ) {
            ToastAnchor()

            Skeleton(
              enabled = model.query.state !is QueryState.Success,
              colors = SkeletonDefaults.inverseColors(),
            ) {
              ContinueWritingNotification(doc = doc, onDismiss = { model.dismissContinueWriting() })
            }
          }
        }
      },
  ) { contentPadding ->
    if (recentDocs.isEmpty()) {
      EmptyHome(
        modifier =
          Modifier.fillMaxSize()
            .padding(contentPadding)
            .padding(bottom = BottomBarDefaults.BarAreaHeight),
        userName = model.query.data.me.name,
        onCreate = createDocument,
      )
    } else {
      FilledHome(
        scrollState = scrollState,
        continueDocs = continueDocs,
        recentDocs = recentDocs,
        siteName = model.query.data.site.name,
        contentPadding =
          contentPadding +
            PaddingValues(
              bottom =
                if (continueWritingDoc != null) ContinueWritingPinHeight + ContinueWritingPinGap
                else 0.dp
            ),
      )
    }

    if (continueWritingDoc == null) {
      ToastAnchor(
        modifier =
          Modifier.align(Alignment.BottomCenter)
            .navigationBarsPadding()
            .padding(bottom = BottomBarDefaults.BarAreaHeight)
      )
    }
  }
}

@Composable
private fun FilledHome(
  scrollState: ScrollState,
  continueDocs: List<HomeScreen_ContinueWriting_document>,
  recentDocs: List<HomeScreen_RecentDocumentRow_document>,
  siteName: String,
  contentPadding: PaddingValues,
) {
  val nav = Nav.current

  val now = remember { Clock.System.now() }
  val dayCutoff = remember(now) { now.minus(24.hours) }
  val active = continueDocs.filter { it.updatedAt > dayCutoff }
  val rest = recentDocs.filter { it.updatedAt <= dayCutoff }

  Column(
    Modifier.fillMaxSize()
      .verticalScroll(scrollState)
      .padding(contentPadding)
      .padding(bottom = BottomBarDefaults.BarAreaHeight)
      .padding(AppTheme.spacings.scrollBottomPadding)
  ) {
    Box(modifier = Modifier.padding(horizontal = 16.dp)) {
      Skeleton.Bone(
        modifier = Modifier.fillMaxWidth().height(HomeSearchBarHeight),
        shape = AppShapes.rounded(AppShapes.md),
      ) {
        SearchBar(
          placeholder = "${siteName.truncate(10)}에서 검색...",
          onClick = { nav.navigate(Route.Search) },
        )
      }
    }

    if (active.isNotEmpty()) {
      Spacer(Modifier.height(28.dp))
      ContinueWritingSection(docs = active)
    }

    if (rest.isNotEmpty()) {
      Spacer(Modifier.height(28.dp))
      RecentDocumentsSection(docs = rest)
    }
  }
}

@Composable
private fun EmptyHome(
  userName: String,
  modifier: Modifier = Modifier,
  onCreate: suspend () -> Unit,
) {
  Column(
    modifier = modifier.padding(horizontal = 32.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.Center,
  ) {
    BlinkingCaret()

    Spacer(Modifier.height(24.dp))

    Text(
      "처음 뵙겠습니다,\n${userName} 님.",
      style = AppTheme.typography.heading.copy(fontFamily = PaperlogyFontFamily),
      color = AppTheme.colors.textDefault,
      textAlign = TextAlign.Center,
    )

    Spacer(Modifier.height(12.dp))

    Text(
      "빈 문서 한 장에서\n오늘의 첫 문장을 시작해보세요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textMuted,
      textAlign = TextAlign.Center,
    )

    Spacer(Modifier.height(28.dp))

    InteractionScope {
      Row(
        modifier =
          Modifier.height(50.dp)
            .widthIn(min = 180.dp)
            .background(AppTheme.colors.textDefault, AppShapes.rounded(AppShapes.full))
            .pressScale()
            .clickable(onClick = onCreate)
            .padding(horizontal = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
      ) {
        Icon(
          icon = Lucide.Plus,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.surfaceDefault,
        )

        Spacer(Modifier.width(8.dp))

        Text("새 문서 쓰기", style = AppTheme.typography.action, color = AppTheme.colors.surfaceDefault)
      }
    }
  }
}

@Composable
private fun BlinkingCaret() {
  var visible by remember { mutableStateOf(true) }

  LaunchedEffect(Unit) {
    while (true) {
      delay(575)
      visible = !visible
    }
  }

  Box(
    modifier =
      Modifier.width(3.dp)
        .height(44.dp)
        .background(
          AppTheme.colors.textDefault.copy(alpha = if (visible) 1f else 0f),
          RoundedCornerShape(1.5.dp),
        )
  )
}

@Composable
private fun SearchBar(placeholder: String, onClick: suspend () -> Unit) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier =
      Modifier.fillMaxWidth()
        .height(HomeSearchBarHeight)
        .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.md))
        .clickable(onClick = onClick)
        .padding(horizontal = 16.dp),
  ) {
    Icon(icon = Lucide.Search, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textHint)

    Spacer(Modifier.width(10.dp))

    Text(
      placeholder,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.action,
      color = AppTheme.colors.textHint,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun ContinueWritingSection(docs: List<HomeScreen_ContinueWriting_document>) {
  Column {
    val pagerState = rememberPagerState(pageCount = { docs.size })

    Row(
      modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
      verticalAlignment = Alignment.Bottom,
    ) {
      Skeleton.Keep {
        Text("이어쓰기", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)
      }

      Spacer(Modifier.weight(1f))

      val text = buildAnnotatedString {
        withStyle(SpanStyle(color = AppTheme.colors.textMuted, fontWeight = FontWeight.Bold)) {
          append((pagerState.currentPage + 1).toString())
        }

        withStyle(SpanStyle(color = AppTheme.colors.textHint)) { append(" / ${docs.size}") }
      }

      Skeleton.Ignore { Text(text, style = AppTheme.typography.caption) }
    }

    Spacer(Modifier.height(12.dp))

    HorizontalPager(state = pagerState, modifier = Modifier.fillMaxWidth()) { page ->
      val progress = pagerState.currentPage + pagerState.currentPageOffsetFraction
      val activeness = (1f - abs(progress - page)).coerceIn(0f, 1f)

      Box(Modifier.fillMaxWidth().padding(horizontal = 16.dp)) {
        ContinueWritingCard(doc = docs[page], activeness = activeness)
      }
    }

    Spacer(Modifier.height(16.dp))

    CarouselDots(pagerState = pagerState, modifier = Modifier.align(Alignment.CenterHorizontally))
  }
}

@Composable
private fun ContinueWritingCard(doc: HomeScreen_ContinueWriting_document, activeness: Float) {
  val nav = Nav.current
  val breadcrumbSegments = doc.entity.ancestors.mapNotNull { it.node.onFolder?.name }
  val net = doc.characterCountChange.additions - doc.characterCountChange.deletions
  val shape = AppShapes.rounded(AppShapes.xl)

  val borderWidth = lerp(1.dp, 1.5.dp, activeness)
  val borderColor = lerp(AppTheme.colors.borderDefault, AppTheme.colors.textDefault, activeness)

  InteractionScope {
    Column(
      modifier =
        Modifier.fillMaxWidth()
          .shadow(AppTheme.shadows.md, shape, alpha = { activeness })
          .background(AppTheme.colors.surfaceDefault, shape)
          .border(borderWidth, borderColor, shape)
          .clickable(onClick = { nav.navigate(Route.Editor(doc.entity.id)) })
          .pressScale()
          .padding(horizontal = 20.dp, vertical = 20.dp)
    ) {
      Row(verticalAlignment = Alignment.CenterVertically) {
        EntityIcon(entity = doc.entity.entityIcon_entity, modifier = Modifier.size(14.dp))

        Spacer(Modifier.width(8.dp))

        BreadcrumbLine(segments = breadcrumbSegments, modifier = Modifier.weight(1f))

        Spacer(Modifier.width(8.dp))

        Text(
          doc.updatedAt.timeAgo(),
          style = AppTheme.typography.micro,
          color = AppTheme.colors.textHint,
          maxLines = 1,
        )
      }

      Spacer(Modifier.height(8.dp))

      Text(
        formatDocumentTitle(doc.title),
        style = AppTheme.typography.heading,
        color = AppTheme.colors.textDefault,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )

      Spacer(Modifier.height(12.dp))

      Text(
        formatEntityExcerpt(doc.excerpt),
        modifier = Modifier.fillMaxWidth(),
        style = AppTheme.typography.body,
        color = AppTheme.colors.textMuted,
        minLines = 4,
        maxLines = 4,
        overflow = TextOverflow.Ellipsis,
      )

      Spacer(Modifier.height(12.dp))

      DashedDivider()

      Spacer(Modifier.height(12.dp))

      Row(verticalAlignment = Alignment.Bottom) {
        Text(
          buildAnnotatedString {
            withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = AppTheme.colors.textMuted)) {
              append(doc.characterCount.comma)
            }

            withStyle(SpanStyle(color = AppTheme.colors.textHint)) { append(" 자") }
          },
          style = AppTheme.typography.caption,
        )

        if (net != 0) {
          Spacer(Modifier.width(14.dp))

          Text(
            buildAnnotatedString {
              withStyle(
                SpanStyle(fontWeight = FontWeight.Bold, color = AppTheme.colors.textMuted)
              ) {
                append(if (net > 0) "+${net.comma}" else net.comma)
              }
              withStyle(SpanStyle(color = AppTheme.colors.textHint)) { append(" 오늘") }
            },
            style = AppTheme.typography.caption,
          )
        }
      }
    }
  }
}

@Composable
private fun CarouselDots(pagerState: PagerState, modifier: Modifier = Modifier) {
  val progress = pagerState.currentPage + pagerState.currentPageOffsetFraction

  Row(modifier = modifier, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
    repeat(pagerState.pageCount) { i ->
      val t = (1f - abs(progress - i)).coerceIn(0f, 1f)
      val width = lerp(6.dp, 20.dp, t)
      val color = lerp(AppTheme.colors.borderDefault, AppTheme.colors.textDefault, t)

      Box(Modifier.width(width).height(6.dp).background(color, AppShapes.circle))
    }
  }
}

private enum class RecentDocumentSort(val label: String) {
  Edited("편집순"),
  Opened("열람순"),
}

@Composable
private fun RecentDocumentsSection(docs: List<HomeScreen_RecentDocumentRow_document>) {
  val nav = Nav.current
  val tabState = LocalTabState.current

  var sortMode by remember { mutableStateOf(RecentDocumentSort.Edited) }
  val sorted =
    when (sortMode) {
      RecentDocumentSort.Edited -> docs.sortedByDescending { it.updatedAt }
      RecentDocumentSort.Opened -> docs
    }

  Column {
    Divider(modifier = Modifier.padding(horizontal = 16.dp))

    Spacer(Modifier.height(16.dp))

    Skeleton.Keep {
      Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        verticalAlignment = Alignment.Bottom,
      ) {
        Text("최근 문서", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

        Spacer(Modifier.width(4.dp))

        Text(
          docs.size.toString(),
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textHint,
        )

        Spacer(Modifier.weight(1f))

        SortToggle(
          mode = sortMode,
          onToggle = {
            sortMode =
              when (sortMode) {
                RecentDocumentSort.Edited -> RecentDocumentSort.Opened
                RecentDocumentSort.Opened -> RecentDocumentSort.Edited
              }
          },
        )
      }
    }

    Spacer(Modifier.height(4.dp))

    Column(modifier = Modifier.padding(horizontal = 16.dp)) {
      sorted.separated(separator = { Divider() }) { recentDocument ->
        val entity = recentDocument.entity.entityRow_entity
        val parent = recentDocument.entity.entityRowParent_entity.parentFolderMeta()
        EntityRow(
          entity = entity,
          contentPadding = PaddingValues(vertical = 16.dp),
          onClick = { nav.navigate(Route.Editor(entity.id)) },
        ) {
          parentMeta(parent)
          title(
            title = formatDocumentTitle(recentDocument.title),
            subtitle = recentDocument.subtitle,
            trailingText = recentDocument.updatedAt.timeAgo(),
          )
          supporting(text = formatEntityExcerpt(recentDocument.excerpt), maxLines = 2)
        }
      }
    }

    Spacer(Modifier.height(16.dp))

    InteractionScope {
      Row(
        modifier =
          Modifier.fillMaxWidth()
            .clickable(onClick = { tabState.onSelectTab(Tab.Space) })
            .pressScale()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.Center,
      ) {
        Text(
          "스페이스에서 모든 문서 보기",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textMuted,
        )

        Icon(Lucide.ArrowRight, modifier = Modifier.size(16.dp))
      }
    }
  }
}

@Composable
private fun SortToggle(mode: RecentDocumentSort, onToggle: suspend () -> Unit) {
  InteractionScope {
    Row(
      modifier =
        Modifier.clickable(onClick = onToggle)
          .pressScale()
          .padding(horizontal = 2.dp, vertical = 4.dp),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(5.dp),
    ) {
      Icon(
        icon = Lucide.ArrowDownUp,
        modifier = Modifier.size(13.dp),
        tint = AppTheme.colors.textHint,
      )

      Text(mode.label, style = AppTheme.typography.caption, color = AppTheme.colors.textHint)
    }
  }
}

@Composable
private fun BreadcrumbLine(segments: List<String>, modifier: Modifier = Modifier) {
  if (segments.isEmpty()) {
    Spacer(modifier)
    return
  }

  val text = buildAnnotatedString {
    segments.forEachIndexed { index, segment ->
      if (index > 0) {
        withStyle(SpanStyle(color = AppTheme.colors.textHint)) { append(" / ") }
      }

      withStyle(
        SpanStyle(
          color =
            if (index == segments.lastIndex) AppTheme.colors.textMuted else AppTheme.colors.textHint
        )
      ) {
        append(segment)
      }
    }
  }

  Text(
    text,
    modifier = modifier,
    style = AppTheme.typography.micro,
    maxLines = 1,
    overflow = TextOverflow.Ellipsis,
  )
}

@Composable
private fun DashedDivider() {
  val color = AppTheme.colors.borderDefault
  Canvas(modifier = Modifier.fillMaxWidth().height(1.dp)) {
    val dash = 3.dp.toPx()
    drawLine(
      color = color,
      start = Offset(0f, size.height / 2f),
      end = Offset(size.width, size.height / 2f),
      strokeWidth = 1.dp.toPx(),
      pathEffect = PathEffect.dashPathEffect(floatArrayOf(dash, dash)),
      cap = StrokeCap.Round,
    )
  }
}

@Composable
private fun ContinueWritingNotification(
  doc: HomeScreen_ContinueWriting_document,
  onDismiss: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val nav = Nav.current
  val density = LocalDensity.current
  val scope = rememberCoroutineScope()
  val shape = AppShapes.rounded(AppShapes.lg)

  val offsetX = remember { Animatable(0f) }
  val alpha = remember { Animatable(1f) }
  var pillWidthPx by remember { mutableStateOf(0) }

  val dragState = rememberDraggableState { delta ->
    scope.launch {
      val next = offsetX.value + delta
      offsetX.snapTo(next)
      val fadeRatio =
        if (pillWidthPx == 0) 0f else (abs(next) / pillWidthPx.toFloat()).coerceIn(0f, 1f)
      alpha.snapTo(1f - fadeRatio * ContinueWritingPinDragFadeStrength)
    }
  }

  InteractionScope {
    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .height(ContinueWritingPinHeight)
          .onSizeChanged { pillWidthPx = it.width }
          .offset { IntOffset(x = offsetX.value.roundToInt(), y = 0) }
          .shadow(AppTheme.shadows.lg, shape, alpha = { alpha.value })
          .graphicsLayer { this.alpha = alpha.value }
          .draggable(
            state = dragState,
            orientation = Orientation.Horizontal,
            onDragStopped = { velocity ->
              if (pillWidthPx > 0) {
                val distanceThreshold = pillWidthPx * 0.35f
                val velocityThreshold = with(density) { 400.dp.toPx() }
                val shouldDismiss =
                  abs(offsetX.value) > distanceThreshold || abs(velocity) > velocityThreshold
                if (shouldDismiss) {
                  val dominant = if (abs(velocity) > velocityThreshold) velocity else offsetX.value
                  val direction = if (dominant >= 0f) 1f else -1f
                  val target = direction * (pillWidthPx + with(density) { 150.dp.toPx() })
                  coroutineScope {
                    launch { offsetX.animateTo(target, tween(200, easing = EaseOut)) }
                    launch { alpha.animateTo(0f, tween(200, easing = EaseOut)) }
                  }
                  onDismiss()
                } else {
                  coroutineScope {
                    launch { offsetX.animateTo(0f, spring()) }
                    launch { alpha.animateTo(1f, spring()) }
                  }
                }
              }
            },
          )
          .background(AppTheme.colors.surfaceInverse, shape)
          .clickable(onClick = { nav.navigate(Route.Editor(doc.entity.id)) })
          .padding(horizontal = 18.dp)
          .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Column(modifier = Modifier.weight(1f)) {
        Skeleton.Keep {
          Text(
            "마지막 글 이어쓰기",
            style = AppTheme.typography.micro,
            color = AppTheme.colors.textHint,
            maxLines = 1,
          )
        }

        Spacer(Modifier.height(2.dp))

        Text(
          formatDocumentTitle(doc.title),
          style = AppTheme.typography.label,
          color = AppTheme.colors.textOnInverse,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }

      Spacer(Modifier.width(12.dp))

      Skeleton.Ignore {
        Icon(
          icon = Lucide.ArrowRight,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textOnInverse,
        )
      }
    }
  }
}
