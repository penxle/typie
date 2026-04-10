package co.typie.overlay

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ui.component.CardDefaults
import co.typie.entity_transfer.EntityTransferSource
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.EntityMoveSheet_Folder_Query
import co.typie.graphql.EntityMoveSheet_Root_Query
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.EntityListFolderRow
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import androidx.lifecycle.viewmodel.compose.viewModel

private const val MOVE_DEPTH_LIMIT_MESSAGE = "폴더의 최대 깊이를 초과했어요"

private val BreadcrumbFadeWidth = 24.dp
private val MoveListFadeHeight = 24.dp

private data class MoveBreadcrumbItem(
  val entityId: String?,
  val label: String,
  val isCurrent: Boolean,
)

private data class MoveDestinationFolder(
  val item: EntityListItem.Folder,
  val depth: Int,
)

private data class MoveDestinationContent(
  val destinationEntityId: String?,
  val destinationDepth: Int,
  val breadcrumbs: List<MoveBreadcrumbItem>,
  val canNavigateUp: Boolean,
  val parentDestinationId: String?,
  val folders: List<MoveDestinationFolder>,
  val lastChildOrder: String?,
)

@Composable
fun BottomSheetScope<Unit>.EntityMoveSheet(
  source: EntityTransferSource,
  onMoved: () -> Unit = {},
) {
  val toast = LocalToast.current
  val model = viewModel(key = "entity-move:${source.id}") { EntityMoveSheetViewModel() }
  var isMoving by remember(source.id) { mutableStateOf(false) }
  var displayedContent by remember(source.id) { mutableStateOf<MoveDestinationContent?>(null) }
  var navigationDirection by remember(source.id) { mutableStateOf(MoveDestinationNavigationDirection.None) }

  LaunchedEffect(source.id) {
    model.showRoot()
  }

  val queryState = if (model.destinationEntityId == null) model.rootQuery.state else model.entityQuery.state
  val destinationContent = when {
    model.destinationEntityId == null ->
      (model.rootQuery.state as? QueryState.Success)?.data?.site?.toDestinationContent()

    else ->
      (model.entityQuery.state as? QueryState.Success)?.data?.entity?.toDestinationContent()
  }
  LaunchedEffect(destinationContent) {
    if (destinationContent != null) {
      displayedContent = destinationContent
    }
  }

  val isLoadingDestination = queryState is QueryState.Loading
  val canSubmit = destinationContent != null && queryState is QueryState.Success && !isMoving
  val isMoveAllowed = destinationContent?.let { source.canTransferIntoDestinationDepth(it.destinationDepth) } == true

  suspend fun submit() {
    val resolvedDestination = destinationContent ?: return
    if (isMoving) return

    if (!isMoveAllowed) {
      toast.show(ToastType.Error, MOVE_DEPTH_LIMIT_MESSAGE)
      return
    }

    isMoving = true
    model.moveEntity(
      source = source,
      parentEntityId = resolvedDestination.destinationEntityId,
      lowerOrder = resolvedDestination.lastChildOrder,
      upperOrder = null,
    )
      .withDefaultExceptionHandler(toast)
      .onOk {
        onMoved()
        dismiss()
      }
    isMoving = false
  }

  fun navigateTo(nextDestinationId: String?) {
    val currentContent = displayedContent ?: destinationContent ?: return
    if (isLoadingDestination || isMoving) return

    navigationDirection = resolveMoveDestinationNavigationDirection(
      currentDestinationId = currentContent.destinationEntityId,
      nextDestinationId = nextDestinationId,
      childDestinationIds = currentContent.folders.mapTo(mutableSetOf()) { it.item.id },
    )
    model.showDestination(nextDestinationId)
  }

  BottomSheetScaffold(
    modifier = Modifier.fillMaxHeight(),
    title = source.transferActionLabel,
    fillAvailableHeight = true,
    scrollContent = false,
    footer = {
      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        Button(
          text = "취소",
          modifier = Modifier.weight(1f),
          variant = ButtonVariant.Secondary,
          enabled = !isMoving,
          onClick = { dismiss() },
        )

        Button(
          text = "여기에 옮기기",
          loadingText = "옮기는 중...",
          modifier = Modifier.weight(1f),
          enabled = canSubmit && isMoveAllowed,
          loading = isMoving,
          onClick = { submit() },
        )
      }
    },
  ) {
    val content = displayedContent
    if (content != null) {
      Text(
        text = "옮길 위치",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )

      MoveBreadcrumbs(
        items = content.breadcrumbs,
        enabled = !isLoadingDestination && !isMoving,
        backgroundColor = AppTheme.colors.surfaceRaised,
        onNavigate = ::navigateTo,
      )
    }

    when (queryState) {
      QueryState.Loading -> {
        if (content != null) {
          MoveDestinationAnimatedBody(
            source = source,
            content = content,
            navigationDirection = navigationDirection,
            navigationEnabled = false,
            onNavigate = ::navigateTo,
          )
        } else {
          MoveSheetStatus(
            message = "폴더를 불러오는 중이에요",
          )
        }
      }

      is QueryState.Error -> {
        MoveSheetError(
          onRetry = { model.refetch() },
        )
      }

      is QueryState.Success -> {
        if (content != null) {
          MoveDestinationAnimatedBody(
            modifier = Modifier
              .fillMaxWidth()
              .weight(1f),
            source = source,
            content = content,
            navigationDirection = navigationDirection,
            navigationEnabled = !isMoving,
            onNavigate = ::navigateTo,
          )
        } else {
          MoveSheetStatus(
            message = "폴더를 불러오는 중이에요",
          )
        }
      }
    }
  }
}

@Composable
private fun MoveDestinationAnimatedBody(
  modifier: Modifier = Modifier,
  source: EntityTransferSource,
  content: MoveDestinationContent,
  navigationDirection: MoveDestinationNavigationDirection,
  navigationEnabled: Boolean,
  onNavigate: (String?) -> Unit,
) {
  AnimatedContent(
    modifier = modifier,
    targetState = content,
    transitionSpec = {
      when (navigationDirection) {
        MoveDestinationNavigationDirection.Forward ->
          slideInHorizontally { it } togetherWith slideOutHorizontally { -it }

        MoveDestinationNavigationDirection.Backward ->
          slideInHorizontally { -it } togetherWith slideOutHorizontally { it }

        MoveDestinationNavigationDirection.None ->
          fadeIn() togetherWith fadeOut()
      }
    },
  ) { currentContent ->
    MoveDestinationBody(
      source = source,
      content = currentContent,
      navigationEnabled = navigationEnabled,
      onNavigate = onNavigate,
    )
  }
}

@Composable
private fun MoveDestinationBody(
  source: EntityTransferSource,
  content: MoveDestinationContent,
  navigationEnabled: Boolean,
  onNavigate: (String?) -> Unit,
) {
  CardSurface(
    modifier = Modifier.fillMaxSize(),
    color = AppTheme.colors.surfaceSunken,
  ) {
    val scrollState = rememberScrollState()
    val showTopFade by remember(scrollState) {
      derivedStateOf { scrollState.value > 0 }
    }
    val showBottomFade by remember(scrollState) {
      derivedStateOf { scrollState.value < scrollState.maxValue }
    }
    val topFadeAlpha by animateFloatAsState(
      targetValue = if (showTopFade) 1f else 0f,
      animationSpec = tween(250),
    )
    val bottomFadeAlpha by animateFloatAsState(
      targetValue = if (showBottomFade) 1f else 0f,
      animationSpec = tween(250),
    )

    Box(modifier = Modifier.fillMaxSize()) {
      Column(
        modifier = Modifier
          .fillMaxSize()
          .verticalScroll(scrollState),
      ) {
        var hasRow = false

        if (content.canNavigateUp) {
          MoveNavigateUpRow(
            enabled = navigationEnabled,
            onClick = { onNavigate(content.parentDestinationId) },
          )
          hasRow = true
        }

        content.folders.forEach { folder ->
          if (hasRow) {
            CardDivider(color = AppTheme.colors.borderDefault)
          }

          EntityListFolderRow(
            item = folder.item,
            enabled = navigationEnabled &&
              folder.item.id != source.id &&
              source.canTransferIntoDestinationDepth(folder.depth),
            onClick = { onNavigate(folder.item.id) },
          )
          hasRow = true
        }

        if (content.folders.isEmpty()) {
          if (hasRow) {
            CardDivider(color = AppTheme.colors.borderDefault)
          }

          MoveSheetStatus(
            message = "하위 폴더가 없어요",
          )
        }
      }

      Box(
        modifier = Modifier.matchParentSize(),
      ) {
        Box(
          modifier = Modifier
            .align(Alignment.TopCenter)
            .fillMaxWidth()
            .height(MoveListFadeHeight),
        ) {
          Box(
            modifier = Modifier
              .matchParentSize()
              .graphicsLayer { alpha = topFadeAlpha }
              .background(
                brush = Brush.verticalGradient(
                  colorStops = arrayOf(
                    0.3f to AppTheme.colors.surfaceSunken.copy(alpha = 0.92f),
                    1f to AppTheme.colors.surfaceSunken.copy(alpha = 0f),
                  ),
                ),
              ),
          )
        }

        Box(
          modifier = Modifier
            .align(Alignment.BottomCenter)
            .fillMaxWidth()
            .height(MoveListFadeHeight),
        ) {
          Box(
            modifier = Modifier
              .matchParentSize()
              .graphicsLayer { alpha = bottomFadeAlpha }
              .background(
                brush = Brush.verticalGradient(
                  colorStops = arrayOf(
                    0f to AppTheme.colors.surfaceSunken.copy(alpha = 0f),
                    0.7f to AppTheme.colors.surfaceSunken.copy(alpha = 0.92f),
                  ),
                ),
              ),
          )
        }
      }
    }
  }
}

@Composable
private fun MoveBreadcrumbs(
  items: List<MoveBreadcrumbItem>,
  enabled: Boolean,
  backgroundColor: androidx.compose.ui.graphics.Color,
  onNavigate: (String?) -> Unit,
) {
  val scrollState = rememberScrollState()
  val showLeftFade by remember(scrollState) {
    derivedStateOf { scrollState.value > 0 }
  }
  val showRightFade by remember(scrollState) {
    derivedStateOf { scrollState.value < scrollState.maxValue }
  }
  val leftFadeAlpha by animateFloatAsState(
    targetValue = if (showLeftFade) 1f else 0f,
    animationSpec = tween(250),
  )
  val rightFadeAlpha by animateFloatAsState(
    targetValue = if (showRightFade) 1f else 0f,
    animationSpec = tween(250),
  )

  LaunchedEffect(items, scrollState.maxValue) {
    if (scrollState.maxValue > 0 && scrollState.value != scrollState.maxValue) {
      scrollState.animateScrollTo(scrollState.maxValue)
    }
  }

  Box(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Row(
      modifier = Modifier.horizontalScroll(scrollState),
      horizontalArrangement = Arrangement.spacedBy(6.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      items.forEachIndexed { index, item ->
        if (index > 0) {
          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textTertiary,
          )
        }

        MoveBreadcrumbChip(
          item = item,
          enabled = enabled,
          onClick = { onNavigate(item.entityId) },
        )
      }
    }

    Box(
      modifier = Modifier.matchParentSize(),
    ) {
      Box(
        modifier = Modifier
          .align(Alignment.CenterStart)
          .fillMaxHeight()
          .width(BreadcrumbFadeWidth),
      ) {
        Box(
          modifier = Modifier
            .matchParentSize()
            .graphicsLayer { alpha = leftFadeAlpha }
            .background(
              brush = Brush.horizontalGradient(
                colorStops = arrayOf(
                  0.3f to backgroundColor.copy(alpha = 0.92f),
                  1f to backgroundColor.copy(alpha = 0f),
                ),
              ),
            ),
        )
      }
      Box(
        modifier = Modifier
          .align(Alignment.CenterEnd)
          .fillMaxHeight()
          .width(BreadcrumbFadeWidth),
      ) {
        Box(
          modifier = Modifier
            .matchParentSize()
            .graphicsLayer { alpha = rightFadeAlpha }
            .background(
              brush = Brush.horizontalGradient(
                colorStops = arrayOf(
                  0f to backgroundColor.copy(alpha = 0f),
                  0.7f to backgroundColor.copy(alpha = 0.92f),
                ),
              ),
            ),
        )
      }
    }
  }
}

@Composable
private fun MoveBreadcrumbChip(
  item: MoveBreadcrumbItem,
  enabled: Boolean,
  onClick: suspend () -> Unit,
) {
  val color = if (item.isCurrent) AppTheme.colors.textPrimary else AppTheme.colors.textTertiary

  if (item.isCurrent || !enabled) {
    Text(
      text = item.label,
      style = AppTheme.typography.action,
      color = color,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    return
  }

  InteractionScope {
    Text(
      text = item.label,
      modifier = Modifier
        .clickable(onClick = onClick)
        .pressScale(0.96f),
      style = AppTheme.typography.action,
      color = color,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun MoveNavigateUpRow(
  enabled: Boolean,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = Modifier
        .fillMaxWidth()
        .alpha(if (enabled) 1f else 0.48f)
        .then(if (enabled) Modifier.clickable(onClick) else Modifier)
        .then(if (enabled) Modifier.pressScale() else Modifier)
        .padding(CardDefaults.RowPadding),
    ) {
      Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        Icon(
          icon = Lucide.CornerLeftUp,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textPrimary,
        )

        Text(
          text = "상위 폴더로 가기",
          style = AppTheme.typography.label,
          color = AppTheme.colors.textPrimary,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }

      Icon(
        icon = Lucide.ChevronRight,
        modifier = Modifier
          .align(Alignment.CenterEnd)
          .size(18.dp),
        tint = AppTheme.colors.textTertiary,
      )
    }
  }
}

@Composable
private fun MoveSheetStatus(
  message: String,
) {
  Box(
    modifier = Modifier
      .fillMaxWidth()
      .height(120.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = message,
      style = AppTheme.typography.action,
      color = AppTheme.colors.textTertiary,
    )
  }
}

@Composable
private fun MoveSheetError(
  onRetry: suspend () -> Unit,
) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    MoveSheetStatus(message = "폴더를 불러오지 못했어요")
    Button(
      text = "다시 시도",
      variant = ButtonVariant.Secondary,
      onClick = onRetry,
    )
  }
}

private fun EntityMoveSheet_Root_Query.Site.toDestinationContent(): MoveDestinationContent {
  return MoveDestinationContent(
    destinationEntityId = null,
    destinationDepth = -1,
    breadcrumbs = listOf(
      MoveBreadcrumbItem(
        entityId = null,
        label = name,
        isCurrent = true,
      ),
    ),
    canNavigateUp = false,
    parentDestinationId = null,
    folders = entities.mapNotNull { it.toMoveDestinationFolder() },
    lastChildOrder = entities.lastOrNull()?.order,
  )
}

private fun EntityMoveSheet_Folder_Query.Entity.toDestinationContent(): MoveDestinationContent {
  val currentName = node.onFolder?.name ?: "폴더"

  return MoveDestinationContent(
    destinationEntityId = id,
    destinationDepth = depth,
    breadcrumbs = buildList {
      add(
        MoveBreadcrumbItem(
          entityId = null,
          label = site.name,
          isCurrent = false,
        ),
      )
      ancestors.forEach { ancestor ->
        val name = ancestor.node.onFolder?.name ?: return@forEach
        add(
          MoveBreadcrumbItem(
            entityId = ancestor.id,
            label = name,
            isCurrent = false,
          ),
        )
      }
      add(
        MoveBreadcrumbItem(
          entityId = id,
          label = currentName,
          isCurrent = true,
        ),
      )
    },
    canNavigateUp = true,
    parentDestinationId = ancestors.lastOrNull()?.id,
    folders = children.mapNotNull { it.toMoveDestinationFolder() },
    lastChildOrder = children.lastOrNull()?.order,
  )
}

private fun EntityMoveSheet_Root_Query.Entity.toMoveDestinationFolder(): MoveDestinationFolder? {
  val folder = node.onFolder ?: return null
  return MoveDestinationFolder(
    item = EntityListItem.Folder(
      id = id,
      folderId = folder.id,
      iconName = icon,
      iconColor = iconColor,
      name = folder.name,
      folderCount = folder.folderCount,
      documentCount = folder.documentCount,
    ),
    depth = depth,
  )
}

private fun EntityMoveSheet_Folder_Query.Child.toMoveDestinationFolder(): MoveDestinationFolder? {
  val folder = node.onFolder ?: return null
  return MoveDestinationFolder(
    item = EntityListItem.Folder(
      id = id,
      folderId = folder.id,
      iconName = icon,
      iconColor = iconColor,
      name = folder.name,
      folderCount = folder.folderCount,
      documentCount = folder.documentCount,
    ),
    depth = depth,
  )
}
