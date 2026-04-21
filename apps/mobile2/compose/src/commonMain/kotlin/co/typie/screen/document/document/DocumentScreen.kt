package co.typie.screen.document.document

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.format
import co.typie.datetime.timeAgo
import co.typie.domain.entity.DocumentEntityShareSheet
import co.typie.domain.entity.EntityBreadcrumb
import co.typie.domain.entity.EntityBreadcrumbLayout
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.EntityIconPickerSheet
import co.typie.domain.entity.EntityIconPickerStopPolicy
import co.typie.domain.entity.EntityIconPickerStops
import co.typie.domain.entity.EntityMoveSheet
import co.typie.domain.entity.EntityMoveStops
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entitytransfer.EntityClipboardService
import co.typie.domain.entitytransfer.EntityTransferSource
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.comma
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.type.DocumentType
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.isOk
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.bleedPadding
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext

private const val DocumentExpandableMetricAnimationDurationMillis = 220
private const val DocumentCharacterCountPlaceholder = 9999

@Composable
fun DocumentScreen(entityId: String) {
  val nav = Nav.current
  val uriHandler = LocalUriHandler.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val model = viewModel { DocumentViewModel() }
  val scrollState = rememberScrollState()
  val loading = model.query.state !is QueryState.Success

  LaunchedEffect(entityId) { model.entityId = entityId }

  val entity = model.query.data.entity
  val document = entity.node.onDocument
  var characterCountExpanded by rememberSaveable(entityId) { mutableStateOf(false) }
  var todayRecordExpanded by rememberSaveable(entityId) { mutableStateOf(false) }

  fun showPendingAction(label: String) {
    toast.show(ToastType.Notification, "$label 기능은 아직 준비 중이에요.")
  }

  fun currentTransferSource(): EntityTransferSource.Document? {
    val resolvedDocument = document ?: return null
    return EntityTransferSource.Document(
      id = entity.id,
      title = formatDocumentTitle(resolvedDocument.title),
      depth = entity.depth,
    )
  }

  suspend fun popDocumentAndMatchingEditorIfPresent(): Boolean {
    val previousRoute = nav.previous
    if (previousRoute !is Route.Editor || previousRoute.entityId != entityId) return false

    val targetRoute = nav.stack.getOrNull(nav.stack.lastIndex - 2)
    if (targetRoute != null) {
      nav.popTo(targetRoute)
    } else {
      nav.pop()
    }
    return true
  }

  suspend fun popAfterDelete() {
    withContext(NonCancellable) {
      if (!popDocumentAndMatchingEditorIfPresent()) {
        nav.pop()
      }
    }
  }

  suspend fun replaceWithDuplicatedEditor(duplicatedEntityId: String) {
    withContext(NonCancellable) {
      popDocumentAndMatchingEditorIfPresent()
      nav.navigate(Route.Editor(duplicatedEntityId))
      toast.success("문서를 복제했어요.")
    }
  }

  LaunchedEffect(loading, document, nav.isTransitioning) {
    if (!loading && document == null && !nav.isTransitioning) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = {
      document?.let { document ->
        val subtitle = document.subtitle?.takeIf(String::isNotBlank)

        Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
          Skeleton.Passive(enabled = loading) {
            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .widthIn(max = ResponsiveContainerDefaults.MaxWidth)
                  .height(TopBarDefaults.TitleHeight)
                  .padding(horizontal = 12.dp),
              contentAlignment = Alignment.CenterStart,
            ) {
              Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
              ) {
                EntityIcon(entity = entity.entityIcon_entity, modifier = Modifier.size(21.dp))

                Spacer(Modifier.width(12.dp))

                Column(
                  modifier = Modifier.weight(1f),
                  verticalArrangement =
                    if (subtitle == null) Arrangement.Center else Arrangement.spacedBy(2.dp),
                ) {
                  Text(
                    text = formatDocumentTitle(document.title),
                    style = AppTheme.typography.title.copy(fontSize = 16.sp),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )

                  if (subtitle != null) {
                    Text(
                      text = subtitle,
                      style = AppTheme.typography.caption.copy(fontSize = 13.sp),
                      color = AppTheme.colors.textMuted,
                      maxLines = 1,
                      overflow = TextOverflow.Ellipsis,
                    )
                  }
                }
              }
            }
          }
        }
      }
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query, background = AppTheme.colors.surfaceCanvas) { contentPadding ->
    val document = document ?: return@Screen
    val subtitle = document.subtitle?.takeIf(String::isNotBlank)
    val createdAt =
      "${document.createdAt.timeAgo()} · ${document.createdAt.format("yyyy.MM.dd HH:mm")}"
    val updatedAt =
      "${document.updatedAt.timeAgo()} · ${document.updatedAt.format("yyyy.MM.dd HH:mm")}"
    val netChange =
      document.characterCountChange.additions - document.characterCountChange.deletions

    val openIconPicker: suspend () -> Unit = {
      if (!loading) {
        sheet.present(stops = EntityIconPickerStops, stopPolicy = EntityIconPickerStopPolicy) {
          EntityIconPickerSheet(
            model = model,
            entityId = entity.id,
            initialIcon = entity.entityIcon_entity.icon,
            initialColor = entity.entityIcon_entity.iconColor,
            defaultIconName =
              if (document.type == DocumentType.TEMPLATE) "layout-template" else "file",
            onUpdated = model::refetch,
          )
        }
      }
    }
    val shareDocument: suspend () -> Unit = {
      if (!loading) {
        sheet.present {
          DocumentEntityShareSheet(entityIds = listOf(entity.id), onUpdated = model::refetch)
        }
      }
    }
    val openInSpace: suspend () -> Unit = {
      if (!loading) {
        entity.url.takeIf(String::isNotBlank)?.let(uriHandler::openUri)
      }
    }
    val moveDocument: suspend () -> Unit = {
      if (!loading) {
        currentTransferSource()?.let { transferSource ->
          sheet.present(stops = EntityMoveStops) {
            EntityMoveSheet(
              source = transferSource,
              initialDestinationId = null,
              onMoved = model::refetch,
            )
          }
        }
      }
    }
    val toggleDocumentType: suspend () -> Unit = {
      if (!loading) {
        val nextType =
          if (document.type == DocumentType.TEMPLATE) DocumentType.NORMAL else DocumentType.TEMPLATE
        val isToTemplate = nextType == DocumentType.TEMPLATE
        val result =
          dialog.confirm(
            title = if (isToTemplate) "템플릿으로 전환" else "문서로 전환",
            message =
              if (isToTemplate) {
                "이 문서를 템플릿으로 전환하시겠어요?\n앞으로 새 문서를 생성할 때 이 문서의 내용을 쉽게 이용할 수 있어요."
              } else {
                "이 템플릿을 다시 일반 문서로 전환하시겠어요?"
              },
            confirmText = "전환",
          )
        if (result is DialogResult.Resolved) {
          model.updateDocumentType(document.id, nextType).withDefaultExceptionHandler(toast).also {
            if (it.isOk) model.refetch()
          }
        }
      }
    }
    val duplicateDocument: suspend () -> Unit = {
      if (!loading) {
        model.duplicateDocument(document.id).withDefaultExceptionHandler(toast).onOk {
          duplicatedEntityId ->
          replaceWithDuplicatedEditor(duplicatedEntityId)
        }
      }
    }
    val copyDocumentToClipboard: suspend () -> Unit = copyDocumentToClipboard@{
      if (!loading) {
        val transferSource = currentTransferSource() ?: return@copyDocumentToClipboard
        val sourceSiteId = Preference.siteId ?: return@copyDocumentToClipboard
        EntityClipboardService.setCopy(sourceSiteId = sourceSiteId, items = listOf(transferSource))
        toast.success("문서를 복사했어요.\n원하는 폴더에 붙여넣을 수 있어요.")
      }
    }
    val cutDocumentToClipboard: suspend () -> Unit = cutDocumentToClipboard@{
      if (!loading) {
        val transferSource = currentTransferSource() ?: return@cutDocumentToClipboard
        val sourceSiteId = Preference.siteId ?: return@cutDocumentToClipboard
        EntityClipboardService.setCut(sourceSiteId = sourceSiteId, items = listOf(transferSource))
        toast.success("문서를 잘라냈어요.\n원하는 폴더에 붙여넣을 수 있어요.")
      }
    }
    val deleteDocument: suspend () -> Unit = {
      if (!loading) {
        val result =
          dialog.confirm(
            title = "문서 삭제",
            message =
              "\"${formatDocumentTitle(document.title)}\" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
            confirmText = "삭제하기",
            confirmIsDestructive = true,
          )
        if (result is DialogResult.Resolved) {
          val deleteResult = model.deleteDocument(document.id).withDefaultExceptionHandler(toast)
          if (deleteResult.isOk) {
            popAfterDelete()
          }
        }
      }
    }

    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(horizontal = 16.dp)
          .padding(bottom = 12.dp)
    ) {
      InteractionScope {
        Box(
          modifier =
            Modifier.size(42.dp)
              .clip(AppShapes.rounded(AppShapes.md))
              .background(AppTheme.colors.surfaceDefault)
              .clickable(onClick = openIconPicker)
              .pressScale(),
          contentAlignment = Alignment.Center,
        ) {
          EntityIcon(entity = entity.entityIcon_entity, modifier = Modifier.size(20.dp))
        }
      }

      Spacer(Modifier.height(12.dp))

      Text(text = formatDocumentTitle(document.title), style = AppTheme.typography.title)

      if (subtitle != null) {
        Spacer(Modifier.height(8.dp))
        Text(
          text = subtitle,
          style = AppTheme.typography.body,
          color = AppTheme.colors.textMuted,
          maxLines = 3,
          overflow = TextOverflow.Ellipsis,
        )
      }

      Spacer(Modifier.height(12.dp))

      EntityBreadcrumb(
        entity = entity.entityBreadcrumb_entity,
        layout = EntityBreadcrumbLayout.FlowWrap,
        color = AppTheme.colors.textHint,
      )

      Spacer(Modifier.height(28.dp))

      DocumentInfoRow(label = "마지막 수정", value = updatedAt)
      Spacer(Modifier.height(8.dp))
      DocumentInfoRow(label = "최초 생성", value = createdAt)

      Spacer(Modifier.height(18.dp))

      DocumentInfoDivider()
      DocumentExpandableMetric(
        icon = Lucide.Type,
        label = "글자 수",
        value = "${document.characterCount.comma}자",
        expanded = characterCountExpanded,
        onToggle = { characterCountExpanded = !characterCountExpanded },
      ) {
        DocumentInfoRow(label = "공백 포함", value = "${DocumentCharacterCountPlaceholder.comma}자")
        Spacer(Modifier.height(8.dp))
        DocumentInfoRow(label = "공백 미포함", value = "${DocumentCharacterCountPlaceholder.comma}자")
        Spacer(Modifier.height(8.dp))
        DocumentInfoRow(label = "공백/부호 미포함", value = "${DocumentCharacterCountPlaceholder.comma}자")
      }
      DocumentInfoDivider()
      DocumentExpandableMetric(
        icon = Lucide.Goal,
        label = "오늘의 기록",
        value = formatCharacterDelta(netChange),
        valueColor = documentNetChangeColor(netChange),
        valueIcon = documentNetChangeIcon(netChange),
        expanded = todayRecordExpanded,
        onToggle = { todayRecordExpanded = !todayRecordExpanded },
      ) {
        val additions = document.characterCountChange.additions
        val deletions = document.characterCountChange.deletions

        DocumentInfoRow(
          label = "변화량",
          value = formatCharacterDelta(netChange),
          valueColor = documentNetChangeColor(netChange),
          valueIcon = documentNetChangeIcon(netChange),
        )
        Spacer(Modifier.height(8.dp))
        DocumentInfoRow(
          label = "입력한 글자",
          value = "${additions.comma}자",
          valueColor = AppTheme.colors.textMuted,
        )
        Spacer(Modifier.height(8.dp))
        DocumentInfoRow(
          label = "지운 글자",
          value = "${deletions.comma}자",
          valueColor = AppTheme.colors.textMuted,
        )
      }

      Box(
        modifier =
          Modifier.fillMaxWidth()
            .bleedPadding(PaddingValues(horizontal = 32.dp))
            .height(12.dp)
            .background(AppTheme.colors.surfaceInset)
      )

      DocumentActionRow(
        icon = Lucide.Blend,
        label = "공유 및 게시",
        supporting = documentVisibilityLabel(entity.visibility),
        onClick = shareDocument,
      )
      DocumentActionRow(
        icon = Lucide.Globe,
        label = "스페이스에서 열기",
        trailingIcon = Lucide.ExternalLink,
        onClick = openInSpace,
      )
      DocumentActionRow(
        icon = Lucide.FileDown,
        label = "파일로 내보내기",
        onClick = { showPendingAction("파일로 내보내기") },
      )

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      DocumentActionRow(
        icon = Lucide.LockKeyhole,
        label = if (document.locked) "편집 잠금 해제" else "편집 잠금",
        onClick = { showPendingAction(if (document.locked) "편집 잠금 해제" else "편집 잠금") },
      )
      DocumentActionRow(
        icon = if (document.type == DocumentType.TEMPLATE) Lucide.File else Lucide.LayoutTemplate,
        label = if (document.type == DocumentType.TEMPLATE) "문서로 전환" else "템플릿으로 전환",
        onClick = toggleDocumentType,
      )

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      DocumentActionRow(icon = Lucide.FolderSymlink, label = "다른 폴더로 옮기기", onClick = moveDocument)
      DocumentActionRow(icon = Lucide.Copy, label = "복제하기", onClick = duplicateDocument)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      DocumentActionRow(
        icon = Lucide.ClipboardCopy,
        label = "복사",
        onClick = copyDocumentToClipboard,
      )
      DocumentActionRow(icon = Lucide.Scissors, label = "잘라내기", onClick = cutDocumentToClipboard)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      DocumentActionRow(
        icon = Lucide.Trash2,
        label = "삭제하기",
        color = AppTheme.colors.danger,
        onClick = deleteDocument,
      )
    }
  }
}

@Composable
private fun DocumentInfoDivider(modifier: Modifier = Modifier) {
  Box(modifier = modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))
}

@Composable
private fun DocumentInfoRow(
  label: String,
  value: String,
  modifier: Modifier = Modifier,
  valueColor: Color = AppTheme.colors.textMuted.copy(alpha = 0.9f),
  valueIcon: IconData? = null,
) {
  Row(
    modifier = modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(16.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Text(
      text = label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textHint,
    )

    Row(
      horizontalArrangement = Arrangement.spacedBy(4.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      valueIcon?.let { icon ->
        Icon(icon = icon, modifier = Modifier.size(14.dp), tint = valueColor)
      }

      Text(
        text = value,
        style = AppTheme.typography.action.copy(fontWeight = FontWeight.W500),
        color = valueColor,
        maxLines = 2,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}

@Composable
private fun DocumentExpandableMetric(
  icon: IconData,
  label: String,
  value: String,
  expanded: Boolean,
  onToggle: () -> Unit,
  modifier: Modifier = Modifier,
  valueColor: Color = AppTheme.colors.textMuted,
  valueIcon: IconData? = null,
  content: @Composable () -> Unit,
) {
  InteractionScope {
    Column(
      modifier =
        modifier.fillMaxWidth().clickable { onToggle() }.pressScale().padding(vertical = 16.dp)
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Row(
          modifier = Modifier.weight(1f),
          horizontalArrangement = Arrangement.spacedBy(4.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(icon = icon, modifier = Modifier.size(14.dp), tint = AppTheme.colors.textHint)
          Text(
            text = label,
            style = AppTheme.typography.action,
            color = AppTheme.colors.textDefault,
          )
          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(15.dp).rotate(if (expanded) 90f else 0f),
            tint = AppTheme.colors.textHint,
          )
        }

        AnimatedVisibility(
          visible = !expanded,
          enter =
            fadeIn(
              animationSpec =
                tween(DocumentExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
            ),
          exit =
            fadeOut(
              animationSpec =
                tween(DocumentExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
            ),
        ) {
          Row(
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            if (valueIcon != null) {
              Icon(icon = valueIcon, modifier = Modifier.size(14.dp), tint = valueColor)
            }

            Text(
              text = value,
              style = AppTheme.typography.action.copy(fontWeight = FontWeight.W500),
              color = valueColor,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          }
        }
      }

      AnimatedVisibility(
        visible = expanded,
        enter =
          fadeIn(
            animationSpec =
              tween(DocumentExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
          ) +
            expandVertically(
              animationSpec =
                tween(DocumentExpandableMetricAnimationDurationMillis, easing = EaseOutCubic),
              expandFrom = Alignment.Top,
            ),
        exit =
          fadeOut(
            animationSpec =
              tween(DocumentExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
          ) +
            shrinkVertically(
              animationSpec =
                tween(DocumentExpandableMetricAnimationDurationMillis, easing = EaseOutCubic),
              shrinkTowards = Alignment.Top,
            ),
      ) {
        Column(modifier = Modifier.fillMaxWidth().padding(top = 10.dp)) { content() }
      }
    }
  }
}

@Composable
private fun DocumentActionRow(
  icon: IconData,
  label: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  supporting: String? = null,
  supportingSecondary: String? = null,
  trailingIcon: IconData? = Lucide.ChevronRight,
  color: Color = AppTheme.colors.textDefault,
) {
  CardRow(
    onClick = onClick,
    modifier = modifier,
    contentPadding = PaddingValues(vertical = 14.dp),
    spacing = 12.dp,
  ) {
    Icon(icon = icon, modifier = Modifier.size(18.dp), tint = color)

    Column(modifier = Modifier.weight(1f)) {
      Text(text = label, style = AppTheme.typography.action, color = color)

      if (supporting != null) {
        Spacer(Modifier.height(4.dp))
        Text(
          text = supporting,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
          maxLines = 2,
          overflow = TextOverflow.Ellipsis,
        )
      }

      if (supportingSecondary != null) {
        Spacer(Modifier.height(4.dp))
        Text(
          text = supportingSecondary,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textHint,
          maxLines = 2,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }

    if (trailingIcon != null) {
      Icon(icon = trailingIcon, modifier = Modifier.size(15.dp), tint = AppTheme.colors.textHint)
    }
  }
}

private fun documentVisibilityLabel(visibility: EntityVisibility?): String {
  return when (visibility) {
    EntityVisibility.PUBLIC -> "공개"
    EntityVisibility.UNLISTED -> "링크 공개"
    EntityVisibility.PRIVATE,
    EntityVisibility.UNKNOWN__,
    null -> "비공개"
  }
}

@Composable
private fun documentNetChangeColor(value: Int): Color {
  return when {
    value > 0 -> AppTheme.colors.success
    value < 0 -> AppTheme.colors.danger
    else -> AppTheme.colors.textMuted
  }
}

private fun formatCharacterDelta(value: Int): String {
  return when {
    value > 0 -> "+${value.comma}자"
    value < 0 -> "-${(-value).comma}자"
    else -> "없음"
  }
}

private fun documentNetChangeIcon(value: Int): IconData? {
  return when {
    value > 0 -> Lucide.TrendingUp
    value < 0 -> Lucide.TrendingDown
    else -> null
  }
}
