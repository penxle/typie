package co.typie.domain.entity

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entitytransfer.EntityTransferSource
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.EntityMoveSheet_Folder_Query
import co.typie.graphql.EntityMoveSheet_Root_Query
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.icons.Lucide
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardDefaults
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ScrollFogInsets
import co.typie.ui.component.Text
import co.typie.ui.component.bleedingScrollFog
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.scrollFog
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.SheetStop
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toPaddingValues
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

private const val MOVE_DEPTH_LIMIT_MESSAGE = "폴더의 최대 깊이를 초과했어요"
private const val MOVE_SHEET_TITLE = "다른 폴더로 옮기기"

private val BreadcrumbFadeWidth = 24.dp
private val MoveListFadeHeight = 24.dp

private data class MoveBreadcrumbItem(
  val entityId: String?,
  val label: String,
  val isCurrent: Boolean,
)

private enum class MoveDestinationNavigationDirection {
  None,
  Forward,
  Backward,
}

private data class MoveDestinationContent(
  val destinationEntityId: String?,
  val destinationDepth: Int,
  val breadcrumbs: List<MoveBreadcrumbItem>,
  val canNavigateUp: Boolean,
  val parentDestinationId: String?,
  val folders: List<EntityRow_entity>,
  val lastChildOrder: String?,
)

private fun resolveMoveDestinationNavigationDirection(
  currentDestinationId: String?,
  nextDestinationId: String?,
  childDestinationIds: Set<String>,
): MoveDestinationNavigationDirection {
  if (currentDestinationId == nextDestinationId) {
    return MoveDestinationNavigationDirection.None
  }

  if (nextDestinationId != null && nextDestinationId in childDestinationIds) {
    return MoveDestinationNavigationDirection.Forward
  }

  return if (currentDestinationId != null) {
    MoveDestinationNavigationDirection.Backward
  } else {
    MoveDestinationNavigationDirection.None
  }
}

internal fun moveSheetViewModelKey(sourceId: String, destinationEntityId: String?): String {
  return "entity-move-sheet:$sourceId:${destinationEntityId ?: "root"}"
}

internal val EntityMoveStops = listOf(SheetStop.Top(64.dp))

@Composable
context(_: SheetScope<Unit>)
fun EntityMoveSheet(
  source: EntityTransferSource,
  initialDestinationId: String? = null,
  onMoved: () -> Unit = {},
) {
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  var destinationEntityId by
    remember(source.id, initialDestinationId) { mutableStateOf(initialDestinationId) }
  val model =
    viewModel(key = moveSheetViewModelKey(source.id, destinationEntityId)) {
      EntityMoveSheetViewModel(destinationEntityId)
    }
  var isMoving by remember(source.id, initialDestinationId) { mutableStateOf(false) }
  var displayedContent by
    remember(source.id, initialDestinationId) { mutableStateOf<MoveDestinationContent?>(null) }
  var navigationDirection by
    remember(source.id, initialDestinationId) {
      mutableStateOf(MoveDestinationNavigationDirection.None)
    }

  val queryState =
    if (destinationEntityId == null) model.rootQuery.state else model.entityQuery.state
  val destinationContent =
    when {
      destinationEntityId == null -> model.rootQuery.data.site.toDestinationContent()
      else -> model.entityQuery.data.entity.toDestinationContent()
    }
  val settledDestinationContent =
    if (queryState is QueryState.Success) {
      destinationContent
    } else {
      null
    }
  LaunchedEffect(settledDestinationContent) {
    if (settledDestinationContent != null) {
      displayedContent = settledDestinationContent
    }
  }

  val isLoadingDestination = queryState is QueryState.Loading
  val visibleContent =
    when {
      settledDestinationContent != null -> settledDestinationContent
      displayedContent != null -> displayedContent
      isLoadingDestination -> destinationContent
      else -> null
    }
  val showLoadingSkeleton = isLoadingDestination && displayedContent == null
  val canSubmit = queryState is QueryState.Success && !isMoving
  val isMoveAllowed =
    settledDestinationContent?.let { source.canMoveToDepth(it.destinationDepth) } == true

  if (queryState is QueryState.Error) {
    LaunchedEffect(queryState.exception) {
      val result =
        dialog.confirm(
          title = "폴더를 불러오지 못했어요",
          message = "잠시 후 다시 시도해주세요.",
          confirmText = "다시 시도",
          cancelText = "닫기",
        )
      if (result is DialogResult.Resolved) {
        model.refetch()
      } else {
        dismiss()
      }
    }
  }

  suspend fun submit() {
    val resolvedDestination = settledDestinationContent ?: return
    if (isMoving) return

    if (!isMoveAllowed) {
      toast.show(ToastType.Error, MOVE_DEPTH_LIMIT_MESSAGE)
      return
    }

    isMoving = true
    model
      .moveEntity(
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
    val currentContent = visibleContent ?: return
    if (isLoadingDestination || isMoving) return

    navigationDirection =
      resolveMoveDestinationNavigationDirection(
        currentDestinationId = currentContent.destinationEntityId,
        nextDestinationId = nextDestinationId,
        childDestinationIds = currentContent.folders.mapTo(mutableSetOf()) { it.id },
      )
    destinationEntityId = nextDestinationId
  }

  SheetLayout(
    modifier = Modifier.fillMaxHeight(),
    fillHeight = true,
    bodyScroll = false,
    header = {
      SheetBar(
        center = {
          Text(
            text = MOVE_SHEET_TITLE,
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
    footer = {
      Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
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
    val content = visibleContent

    Column(
      modifier = Modifier.fillMaxWidth().weight(1f),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      if (content != null) {
        Text(
          text = "옮길 위치",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )

        Skeleton(enabled = showLoadingSkeleton) {
          MoveBreadcrumbs(
            items = content.breadcrumbs,
            enabled = !isLoadingDestination && !isMoving,
            backgroundColor = AppTheme.colors.surfaceRaised,
            onNavigate = ::navigateTo,
          )
        }
      }

      CardSurface(
        modifier = Modifier.fillMaxWidth().weight(1f),
        color = AppTheme.colors.surfaceSunken,
      ) {
        when {
          showLoadingSkeleton && content != null -> {
            Skeleton(enabled = true) {
              MoveDestinationBody(
                modifier = Modifier.fillMaxSize(),
                source = source,
                content = content,
                navigationEnabled = true,
                onNavigate = ::navigateTo,
              )
            }
          }

          content != null -> {
            MoveDestinationAnimatedBody(
              modifier = Modifier.fillMaxSize(),
              source = source,
              content = content,
              navigationDirection = navigationDirection,
              navigationEnabled = queryState is QueryState.Success && !isMoving,
              onNavigate = ::navigateTo,
            )
          }

          else -> {
            Box(modifier = Modifier.fillMaxSize())
          }
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

        MoveDestinationNavigationDirection.None -> fadeIn() togetherWith fadeOut()
      }
    },
    label = "entity-move-destination",
  ) { currentContent ->
    MoveDestinationBody(
      modifier = Modifier.fillMaxSize(),
      source = source,
      content = currentContent,
      navigationEnabled = navigationEnabled,
      onNavigate = onNavigate,
    )
  }
}

@Composable
private fun MoveDestinationBody(
  modifier: Modifier = Modifier,
  source: EntityTransferSource,
  content: MoveDestinationContent,
  navigationEnabled: Boolean,
  onNavigate: (String?) -> Unit,
) {
  val listFogInsets = remember {
    ScrollFogInsets(top = MoveListFadeHeight, bottom = MoveListFadeHeight)
  }
  val scrollState = rememberScrollState()
  val visibleFolders = content.folders.filter { it.folder != null }

  Box(
    modifier =
      modifier
        .fillMaxSize()
        .scrollFog(insets = listFogInsets, color = AppTheme.colors.surfaceSunken)
  ) {
    Column(
      modifier =
        Modifier.fillMaxSize().verticalScroll(scrollState).padding(listFogInsets.toPaddingValues())
    ) {
      var hasRow = false

      if (content.canNavigateUp) {
        MoveNavigateUpRow(
          enabled = navigationEnabled,
          onClick = { onNavigate(content.parentDestinationId) },
        )
        hasRow = true
      }

      visibleFolders.forEach { folder ->
        if (hasRow) {
          CardDivider(color = AppTheme.colors.borderDefault)
        }

        MoveDestinationFolderRow(
          folder = folder,
          enabled =
            navigationEnabled && folder.id != source.id && source.canMoveToDepth(folder.depth),
          onClick = { onNavigate(folder.id) },
        )
        hasRow = true
      }

      if (visibleFolders.isEmpty()) {
        if (hasRow) {
          CardDivider(color = AppTheme.colors.borderDefault)
        }

        MoveSheetStatus(message = "하위 폴더가 없어요")
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
  val breadcrumbFogInsets = remember {
    ScrollFogInsets(left = BreadcrumbFadeWidth, right = BreadcrumbFadeWidth)
  }
  val scrollState = rememberScrollState()

  LaunchedEffect(items, scrollState.maxValue) {
    if (scrollState.maxValue > 0 && scrollState.value != scrollState.maxValue) {
      scrollState.animateScrollTo(scrollState.maxValue)
    }
  }

  Box(
    modifier =
      Modifier.fillMaxWidth()
        .bleedingScrollFog(insets = breadcrumbFogInsets, color = backgroundColor)
  ) {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .horizontalScroll(scrollState)
          .padding(breadcrumbFogInsets.toPaddingValues()),
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

        MoveBreadcrumbChip(item = item, enabled = enabled, onClick = { onNavigate(item.entityId) })
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
      modifier = Modifier.clickable(onClick = onClick).pressScale(0.96f),
      style = AppTheme.typography.action,
      color = color,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun MoveNavigateUpRow(enabled: Boolean, onClick: suspend () -> Unit) {
  InteractionScope {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .alpha(if (enabled) 1f else 0.48f)
          .then(if (enabled) Modifier.clickable(onClick) else Modifier)
          .then(if (enabled) Modifier.pressScale() else Modifier)
          .padding(CardDefaults.RowPadding)
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
        modifier = Modifier.align(Alignment.CenterEnd).size(18.dp),
        tint = AppTheme.colors.textTertiary,
      )
    }
  }
}

@Composable
private fun MoveDestinationFolderRow(
  folder: EntityRow_entity,
  enabled: Boolean,
  onClick: suspend () -> Unit,
) {
  val folderNode = folder.folder ?: return
  EntityRow(
    entity = folder,
    enabled = enabled,
    trailing = { EntityRowChevron() },
    onClick = onClick,
  ) {
    folderTitle(folderNode)
    folderSummary(folderNode)
  }
}

@Composable
private fun MoveSheetStatus(message: String) {
  Box(modifier = Modifier.fillMaxWidth().height(120.dp), contentAlignment = Alignment.Center) {
    Text(text = message, style = AppTheme.typography.action, color = AppTheme.colors.textTertiary)
  }
}

@Composable
private fun MoveSheetError(onRetry: suspend () -> Unit) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    MoveSheetStatus(message = "폴더를 불러오지 못했어요")
    Button(text = "다시 시도", variant = ButtonVariant.Secondary, onClick = onRetry)
  }
}

private fun EntityMoveSheet_Root_Query.Site.toDestinationContent(): MoveDestinationContent {
  return MoveDestinationContent(
    destinationEntityId = null,
    destinationDepth = -1,
    breadcrumbs = listOf(MoveBreadcrumbItem(entityId = null, label = name, isCurrent = true)),
    canNavigateUp = false,
    parentDestinationId = null,
    folders =
      entities.mapNotNull { entity -> entity.entityRow_entity.takeIf { it.folder != null } },
    lastChildOrder = entities.lastOrNull()?.entityRow_entity?.order,
  )
}

private fun EntityMoveSheet_Folder_Query.Entity.toDestinationContent(): MoveDestinationContent {
  val currentName = node.onFolder?.name ?: "폴더"

  return MoveDestinationContent(
    destinationEntityId = id,
    destinationDepth = depth,
    breadcrumbs =
      buildList {
        add(MoveBreadcrumbItem(entityId = null, label = site.name, isCurrent = false))
        ancestors.forEach { ancestor ->
          val name = ancestor.node.onFolder?.name ?: return@forEach
          add(MoveBreadcrumbItem(entityId = ancestor.id, label = name, isCurrent = false))
        }
        add(MoveBreadcrumbItem(entityId = id, label = currentName, isCurrent = true))
      },
    canNavigateUp = true,
    parentDestinationId = ancestors.lastOrNull()?.id,
    folders = children.mapNotNull { child -> child.entityRow_entity.takeIf { it.folder != null } },
    lastChildOrder = children.lastOrNull()?.entityRow_entity?.order,
  )
}
