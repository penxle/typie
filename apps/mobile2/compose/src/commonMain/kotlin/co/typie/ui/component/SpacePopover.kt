package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.SpacePopover_CreateSite_Mutation
import co.typie.graphql.SpacePopover_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildSite
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.text
import co.typie.graphql.type.CreateSiteInput
import co.typie.graphql.watchQuery
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.result
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

val SpacePopoverLeadingKey = Any()

private val SpacePopoverAnchorHeight = 44.dp
private val SpacePopoverAnchorMaxWidth = 200.dp
private val SpacePopoverAnchorLogoSize = 24.dp
private val SpacePopoverAnchorCornerRadius = AppShapes.md
private val SpacePopoverAnchorChevronSize = 14.dp
private val SpacePopoverAnchorRowGap = 8.dp

private val SpacePopoverVerticalOffset = (TopBarDefaults.Height - SpacePopoverAnchorHeight) / 2
private val SpacePopoverScreenPadding =
  PaddingValues(
    start = TopBarDefaults.HorizontalPadding,
    top = SpacePopoverVerticalOffset,
    end = TopBarDefaults.HorizontalPadding,
    bottom = SpacePopoverVerticalOffset + 100.dp,
  )

class SpacePopoverViewModel : ViewModel() {

  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      SpacePopover_Query()
    }
  var isCreatingSite by mutableStateOf(false)
  var pendingCreatedSiteId by mutableStateOf<String?>(null)
    private set

  suspend fun createSite(name: String): Result<Unit, Nothing> {
    if (isCreatingSite) {
      return Result.Ok(Unit)
    }

    isCreatingSite = true
    return result<Unit, Nothing> {
        val data =
          Apollo.executeMutation(
            SpacePopover_CreateSite_Mutation(
              input = CreateSiteInput(name = name.trim().ifBlank { "새 스페이스" })
            )
          )

        pendingCreatedSiteId = data.createSite.id
        query.refetch()
      }
      .also { isCreatingSite = false }
  }

  fun consumePendingCreatedSiteSelection(siteId: String) {
    if (pendingCreatedSiteId == siteId) {
      pendingCreatedSiteId = null
    }
  }
}

@Composable
fun SpacePopover() {
  val model = viewModel { SpacePopoverViewModel() }

  val selectedSiteId = Preference.siteId

  LaunchedEffect(selectedSiteId) {
    if (!selectedSiteId.isNullOrBlank()) {
      model.query.refetch()
    }
  }

  Skeleton(enabled = model.query.state !is QueryState.Success) {
    val data = model.query.data
    val availableSiteIds = data.me.sites.map { it.id }
    val selection =
      resolveSpacePopoverSelection(
        selectedSiteId = Preference.siteId.orEmpty(),
        availableSiteIds = availableSiteIds,
      )

    val currentSite = data.me.sites.first { it.id == selection.currentSiteId }

    if (model.query.state !is QueryState.Success) {
      SpacePopoverAnchor(site = currentSite)
      return@Skeleton
    }

    val otherSites = data.me.sites.filter { it.id in selection.otherSiteIds }

    val pendingCreatedSiteId =
      resolvePendingCreatedSiteSelection(
        pendingCreatedSiteId = model.pendingCreatedSiteId,
        availableSiteIds = availableSiteIds,
      )

    if (pendingCreatedSiteId != null) {
      LaunchedEffect(pendingCreatedSiteId) {
        Preference.siteId = pendingCreatedSiteId
        model.consumePendingCreatedSiteSelection(pendingCreatedSiteId)
      }
    } else if (selection.currentSiteId != Preference.siteId) {
      LaunchedEffect(selection.currentSiteId) { Preference.siteId = selection.currentSiteId }
    }

    val nav = Nav.current
    val uriHandler = LocalUriHandler.current
    val scope = rememberCoroutineScope()
    val sheet = LocalSheet.current

    PopoverMenu(
      placement = PopoverPlacement.BelowStart,
      screenPadding = SpacePopoverScreenPadding,
      collapsedCornerRadius = 12.dp,
      anchor = { SpacePopoverAnchor(currentSite) },
    ) {
      static {
        SpacePopoverHeader(currentSite)
        Spacer(Modifier.height(4.dp))
      }
      item(icon = Lucide.Settings, label = "스페이스 설정") {
        scope.launch { nav.navigate(Route.SpaceSettings) }
      }
      item(icon = Lucide.ExternalLink, label = "스페이스 열기") { uriHandler.openUri(currentSite.url) }
      item(icon = Lucide.Trash2, label = "휴지통") { scope.launch { nav.navigate(Route.Trash()) } }
      divider()
      static {
        Text(
          "다른 스페이스",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.padding(horizontal = 8.dp),
        )
        Spacer(Modifier.height(8.dp))
      }
      for (site in otherSites) {
        item(content = { SpacePopoverSiteItem(site) }) { Preference.siteId = site.id }
      }
      item(icon = Lucide.Plus, label = "새 스페이스 생성") {
        scope.launch { sheet.present { CreateSpaceContent(model) } }
      }
    }
  }
}

@Composable
private fun SpacePopoverAnchor(site: SpacePopover_Query.Site) {
  val shape = AppShapes.squircle(SpacePopoverAnchorCornerRadius)
  val logoShape = AppShapes.rounded(AppShapes.sm)

  Row(
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(SpacePopoverAnchorRowGap),
    modifier =
      Modifier.height(SpacePopoverAnchorHeight)
        .widthIn(max = SpacePopoverAnchorMaxWidth)
        .clip(shape)
        .background(TopBarDefaults.controlBackgroundColor(), shape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), shape)
        .padding(horizontal = 10.dp),
  ) {
    Img(
      image = site.logo.img_image,
      modifier = Modifier.size(SpacePopoverAnchorLogoSize).clip(logoShape),
    )
    Text(
      text = site.name,
      style = AppTheme.typography.label,
      color = AppTheme.colors.textDefault,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
      modifier = Modifier.weight(1f, fill = false),
    )
    Icon(
      icon = Lucide.ChevronDown,
      tint = AppTheme.colors.textMuted,
      modifier = Modifier.size(SpacePopoverAnchorChevronSize),
    )
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun CreateSpaceContent(model: SpacePopoverViewModel) {
  var name by remember { mutableStateOf("") }
  val toast = LocalToast.current

  SheetLayout(
    bodyScroll = false,
    header = {
      SheetBar(
        center = {
          Text(
            text = "새 스페이스 생성",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
  ) {
    Text(
      text = "스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textMuted,
    )

    TextField(
      value = name,
      onValueChange = { name = it },
      label = "스페이스 이름",
      labelPosition = LabelPosition.External,
      placeholder = "새 스페이스",
      autoFocus = true,
    )

    Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
      Button(
        text = "취소",
        variant = ButtonVariant.Secondary,
        enabled = !model.isCreatingSite,
        onClick = { dismiss() },
        modifier = Modifier.weight(1f),
      )

      Button(
        text = "생성",
        loading = model.isCreatingSite,
        enabled = !model.isCreatingSite,
        onClick = {
          model.createSite(name).withDefaultExceptionHandler(toast).onOk {
            toast.show(ToastType.Success, "새 스페이스가 생성되었어요.")
            dismiss()
          }
        },
        modifier = Modifier.weight(1f),
      )
    }
  }
}

@Composable
private fun SpacePopoverHeader(site: SpacePopover_Query.Site) {
  Box(modifier = Modifier.fillMaxWidth().height(TopBarDefaults.ButtonSize)) {
    Box(
      contentAlignment = Alignment.CenterStart,
      modifier = Modifier.fillMaxSize().padding(start = 46.dp, end = 16.dp),
    ) {
      Text(
        site.name,
        style = AppTheme.typography.label,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }

    Box(modifier = Modifier.align(Alignment.CenterStart).padding(start = 8.dp)) {
      SpacePopoverLogo(logo = site.logo.img_image, size = 26.dp)
    }
  }
}

@Composable
private fun SpacePopoverLogo(logo: Img_image, size: Dp) {
  val logoShape = AppShapes.rounded(AppShapes.sm)

  Img(image = logo, modifier = Modifier.size(size).clip(logoShape))
}

@Composable
private fun SpacePopoverSiteItem(site: SpacePopover_Query.Site) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = Modifier.height(48.dp).padding(horizontal = 16.dp),
  ) {
    SpacePopoverLogo(logo = site.logo.img_image, size = 28.dp)
    Spacer(Modifier.width(12.dp))
    Text(
      site.name,
      style = AppTheme.typography.action,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

private fun placeholderData() =
  SpacePopover_Query.Data(PlaceholderResolver) {
    me = buildUser { sites = List(1) { buildSite { name = text(5..10) } } }
  }
