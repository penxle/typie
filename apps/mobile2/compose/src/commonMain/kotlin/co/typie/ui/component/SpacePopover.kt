package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.Apollo
import co.typie.graphql.QueryState
import co.typie.graphql.SpacePopover_CreateSite_Mutation
import co.typie.graphql.SpacePopover_Query
import co.typie.graphql.executeMutation
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.type.CreateSiteInput
import co.typie.graphql.watchQuery
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.result
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.storage.Vault
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.PopoverTransitionElement
import co.typie.ui.component.popover.PopoverTransitionFrame
import co.typie.ui.component.popover.close
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.shape.SquircleShape
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.skeleton.SkeletonBone
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

val SpacePopoverLeadingKey = Any()
private val SpacePopoverVerticalOffset = (TopBarDefaults.Height - TopBarDefaults.ButtonSize) / 2
private val SpacePopoverScreenPadding =
  PaddingValues(
    start = TopBarDefaults.HorizontalPadding,
    top = SpacePopoverVerticalOffset,
    end = TopBarDefaults.HorizontalPadding,
    bottom = SpacePopoverVerticalOffset + 100.dp,
  )

class SpacePopoverViewModel : ViewModel() {

  val query = Apollo.watchQuery(scope = viewModelScope) { SpacePopover_Query() }
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
  val sessionKey = Vault.authTokens?.sessionToken ?: "no-session"
  val model = viewModel(key = "space-popover:$sessionKey") { SpacePopoverViewModel() }

  val selectedSiteId = Preference.siteId

  LaunchedEffect(selectedSiteId) {
    if (!selectedSiteId.isNullOrBlank()) {
      model.query.refetch()
    }
  }

  Skeleton(enabled = model.query.state !is QueryState.Success) {
    when (val state = model.query.state) {
      is QueryState.Success -> {
        val availableSiteIds = state.data.me.sites.map { it.id }
        val selection =
          resolveSpacePopoverSelection(
            selectedSiteId = Preference.siteId.orEmpty(),
            availableSiteIds = availableSiteIds,
          )

        if (selection == null) {
          SpacePopoverSkeleton()
          return@Skeleton
        }

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

        val currentSite = state.data.me.sites.first { it.id == selection.currentSiteId }
        val otherSites = state.data.me.sites.filter { it.id in selection.otherSiteIds }

        Popover(
          placement = PopoverPlacement.BelowStart,
          screenPadding = SpacePopoverScreenPadding,
          collapsedCornerRadius = 14.dp,
          anchor = { SpacePopoverAnchor(currentSite) },
          pane = { SpacePopoverPane(model, currentSite, otherSites) },
        )
      }

      else -> SpacePopoverSkeleton()
    }
  }
}

@Composable
fun SpacePopoverSkeleton() {
  val outerShape = SquircleShape(14.dp)
  val logoShape = RoundedCornerShape(6.dp)

  Box(
    contentAlignment = Alignment.Center,
    modifier =
      Modifier.size(TopBarDefaults.ButtonSize)
        .then(TopBarDefaults.controlShadowModifier(outerShape))
        .clip(outerShape)
        .background(TopBarDefaults.controlBackgroundColor(), outerShape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), outerShape),
  ) {
    SkeletonBone(modifier = Modifier.size(26.dp), shape = logoShape)
  }
}

@Composable
private fun SpacePopoverAnchor(site: SpacePopover_Query.Site) {
  val outerShape = SquircleShape(14.dp)
  val logoShape = RoundedCornerShape(6.dp)

  Box(
    contentAlignment = Alignment.Center,
    modifier =
      Modifier.size(TopBarDefaults.ButtonSize)
        .then(TopBarDefaults.controlShadowModifier(outerShape))
        .clip(outerShape)
        .background(TopBarDefaults.controlBackgroundColor(), outerShape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), outerShape),
  ) {
    Img(image = site.logo.img_image, modifier = Modifier.size(26.dp).clip(logoShape))
  }
}

@Composable
context(_: PopoverScope)
private fun SpacePopoverPane(
  model: SpacePopoverViewModel,
  currentSite: SpacePopover_Query.Site,
  otherSites: List<SpacePopover_Query.Site>,
) {
  val nav = Nav.current
  val uriHandler = LocalUriHandler.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val panePadding = PopoverDefaults.PanePadding

  Column(modifier = Modifier.padding(panePadding)) {
    SpacePopoverHeader(currentSite)

    Spacer(Modifier.height(4.dp))

    PopoverList(
      items =
        listOf(
          PopoverListItem(
            content = { SpacePopoverItem(icon = Lucide.Settings, label = "스페이스 설정") },
            onSelected = {
              close()
              scope.launch { nav.navigate(Route.SpaceSettings) }
            },
          ),
          PopoverListItem(
            content = { SpacePopoverItem(icon = Lucide.ExternalLink, label = "스페이스 열기") },
            onSelected = {
              close()
              uriHandler.openUri(currentSite.url)
            },
          ),
          PopoverListItem(
            content = { SpacePopoverItem(icon = Lucide.Trash2, label = "휴지통") },
            onSelected = {
              close()
              scope.launch { nav.navigate(Route.Trash()) }
            },
          ),
        )
    )

    Spacer(Modifier.height(12.dp))

    Box(
      Modifier.fillMaxWidth()
        .height(1.dp)
        .padding(horizontal = 8.dp)
        .background(AppTheme.colors.borderSubtle)
    )

    Spacer(Modifier.height(12.dp))

    Text(
      "다른 스페이스",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.padding(horizontal = 8.dp),
    )

    Spacer(Modifier.height(8.dp))

    PopoverList(
      items =
        buildList {
          for (site in otherSites) {
            add(
              PopoverListItem(
                content = { SpacePopoverSiteItem(site) },
                onSelected = {
                  Preference.siteId = site.id
                  close()
                },
              )
            )
          }
          add(
            PopoverListItem(
              content = { SpacePopoverItem(icon = Lucide.Plus, label = "새 스페이스 생성") },
              onSelected = {
                close()
                scope.launch { sheet.present { CreateSpaceContent(model) } }
              },
            )
          )
        }
    )
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun CreateSpaceContent(model: SpacePopoverViewModel) {
  var name by remember { mutableStateOf("") }
  val toast = LocalToast.current

  SheetLayout(bodyScroll = false, header = { ActionHeader(title = "새 스페이스 생성") }) {
    Text(
      text = "스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
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

    PopoverTransitionElement(
      collapsedFrame =
        PopoverTransitionFrame(
          left = 9.dp,
          top = (TopBarDefaults.ButtonSize - 26.dp) / 2,
          width = 26.dp,
          height = 26.dp,
        ),
      expandedFrame =
        PopoverTransitionFrame(
          left = 8.dp,
          top = (TopBarDefaults.ButtonSize - 26.dp) / 2,
          width = 26.dp,
          height = 26.dp,
        ),
    ) {
      SpacePopoverLogo(logo = site.logo.img_image, size = 26.dp)
    }
  }
}

@Composable
private fun SpacePopoverLogo(logo: Img_image, size: Dp) {
  val logoShape = RoundedCornerShape(6.dp)

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

@Composable
private fun SpacePopoverItem(icon: IconData, label: String) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
  ) {
    Icon(icon = icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textPrimary)
    Spacer(Modifier.width(12.dp))
    Text(label, style = AppTheme.typography.action, maxLines = 1, overflow = TextOverflow.Ellipsis)
  }
}
