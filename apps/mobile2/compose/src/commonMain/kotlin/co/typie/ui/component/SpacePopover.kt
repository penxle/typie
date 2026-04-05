package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.border
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.auth.AuthService
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.QueryState
import co.typie.graphql.SpacePopover_CreateSite_Mutation
import co.typie.graphql.SpacePopover_Query
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.type.CreateSiteInput
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.service.SiteService
import co.typie.ui.component.bottomsheet.BottomSheetHostState
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.PopoverTransitionElement
import co.typie.ui.component.popover.PopoverTransitionFrame
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.shape.SquircleShape
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.skeleton.SkeletonBone
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel
import org.koin.core.annotation.KoinViewModel

val SpacePopoverLeadingKey = Any()
private val SpacePopoverVerticalOffset = (TopBarDefaults.Height - TopBarDefaults.ButtonSize) / 2
private val SpacePopoverScreenPadding = PaddingValues(
  start = TopBarDefaults.HorizontalPadding,
  top = SpacePopoverVerticalOffset,
  end = TopBarDefaults.HorizontalPadding,
  bottom = SpacePopoverVerticalOffset + 100.dp,
)

@KoinViewModel
class SpacePopoverViewModel(
  private val toast: Toast,
) : GraphQLViewModel() {
  val query = watchQuery { SpacePopover_Query() }
  var isCreatingSite by mutableStateOf(false)
  var pendingCreatedSiteId by mutableStateOf<String?>(null)
    private set

  suspend fun createSite(name: String): Boolean {
    if (isCreatingSite) {
      return false
    }

    isCreatingSite = true
    return try {
      val result = executeMutation(
        SpacePopover_CreateSite_Mutation(
          input = CreateSiteInput(name = name.trim().ifBlank { "새 스페이스" }),
        ),
      )

      pendingCreatedSiteId = result.createSite.id
      query.refetch()
      toast.show(ToastType.Success, "새 스페이스가 생성되었어요.")
      true
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      false
    } finally {
      isCreatingSite = false
    }
  }

  fun consumePendingCreatedSiteSelection(siteId: String) {
    if (pendingCreatedSiteId == siteId) {
      pendingCreatedSiteId = null
    }
  }
}

internal fun showBottomSheetFromPopoverAction(
  closePopover: () -> Unit,
  presenterScope: CoroutineScope,
  bottomSheetHost: BottomSheetHostState,
  content: @Composable BottomSheetScope<Unit>.() -> Unit,
) {
  closePopover()
  presenterScope.launch(start = CoroutineStart.UNDISPATCHED) {
    bottomSheetHost.show(content)
  }
}

@Composable
fun SpacePopover() {
  val authService = koinInject<AuthService>()
  val sessionKey = authService.tokens?.sessionToken ?: "no-session"
  val model = koinViewModel<SpacePopoverViewModel>(key = "space-popover:$sessionKey")
  val siteService = koinInject<SiteService>()
  val presenterScope = rememberCoroutineScope()
  val selectedSiteId = siteService.siteId

  LaunchedEffect(selectedSiteId) {
    if (selectedSiteId.isNotBlank()) {
      model.query.refetch()
    }
  }

  Skeleton(enabled = model.query.state !is QueryState.Success) {
    when (val state = model.query.state) {
      is QueryState.Success -> {
        val availableSiteIds = state.data.me.sites.map { it.id }
        val selection = resolveSpacePopoverSelection(
          selectedSiteId = siteService.siteId,
          availableSiteIds = availableSiteIds,
        )

        if (selection == null) {
          SpacePopoverSkeleton()
          return@Skeleton
        }

        val pendingCreatedSiteId = resolvePendingCreatedSiteSelection(
          pendingCreatedSiteId = model.pendingCreatedSiteId,
          availableSiteIds = availableSiteIds,
        )

        if (pendingCreatedSiteId != null) {
          LaunchedEffect(pendingCreatedSiteId) {
            siteService.siteId = pendingCreatedSiteId
            model.consumePendingCreatedSiteSelection(pendingCreatedSiteId)
          }
        } else if (selection.currentSiteId != siteService.siteId) {
          LaunchedEffect(selection.currentSiteId) {
            siteService.siteId = selection.currentSiteId
          }
        }

        val currentSite = state.data.me.sites.first { it.id == selection.currentSiteId }
        val otherSites = state.data.me.sites.filter { it.id in selection.otherSiteIds }

        Popover(
          placement = PopoverPlacement.BelowStart,
          screenPadding = SpacePopoverScreenPadding,
          collapsedCornerRadius = 14.dp,
          anchor = { SpacePopoverAnchor(currentSite) },
          pane = { SpacePopoverPane(model, presenterScope, currentSite, otherSites) },
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
    modifier = Modifier.size(TopBarDefaults.ButtonSize)
      .then(TopBarDefaults.controlShadowModifier(outerShape)).clip(outerShape)
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
    modifier = Modifier.size(TopBarDefaults.ButtonSize)
      .then(TopBarDefaults.controlShadowModifier(outerShape)).clip(outerShape)
      .background(TopBarDefaults.controlBackgroundColor(), outerShape)
      .border(1.dp, TopBarDefaults.controlBorderColor(), outerShape),
  ) {
    Img(
      image = site.logo.img_image,
      modifier = Modifier.size(26.dp).clip(logoShape),
    )
  }
}

@Composable
private fun PopoverScope.SpacePopoverPane(
  model: SpacePopoverViewModel,
  presenterScope: CoroutineScope,
  currentSite: SpacePopover_Query.Site,
  otherSites: List<SpacePopover_Query.Site>,
) {
  val nav = Nav.current
  val scope = rememberCoroutineScope()
  val siteService = koinInject<SiteService>()
  val bottomSheetHost = LocalBottomSheetHost.current
  val panePadding = PopoverDefaults.PanePadding

  Column(
    modifier = Modifier.padding(panePadding),
  ) {
    SpacePopoverHeader(currentSite)

    Spacer(Modifier.height(4.dp))

    PopoverList(
      items = listOf(
        PopoverListItem(
          content = { SpacePopoverItem(icon = Lucide.Settings, label = "스페이스 설정") },
          onSelected = {
            close()
            scope.launch { nav.navigate(Route.SpaceSettings) }
          },
        ),
        PopoverListItem(
          content = { SpacePopoverItem(icon = Lucide.Trash2, label = "휴지통") },
          onSelected = {
            close()
            scope.launch { nav.navigate(Route.Trash()) }
          },
        ),
      ),
    )

    Spacer(Modifier.height(12.dp))

    Box(
      Modifier.fillMaxWidth().height(1.dp).padding(horizontal = 8.dp)
        .background(AppTheme.colors.borderSubtle),
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
      items = buildList {
        for (site in otherSites) {
          add(
            PopoverListItem(
              content = { SpacePopoverSiteItem(site) },
              onSelected = {
                siteService.siteId = site.id
                close()
              },
            ),
          )
        }
        add(
          PopoverListItem(
            content = { SpacePopoverItem(icon = Lucide.Plus, label = "새 스페이스 생성") },
            onSelected = {
              showBottomSheetFromPopoverAction(
                closePopover = { close() },
                presenterScope = presenterScope,
                bottomSheetHost = bottomSheetHost,
              ) {
                CreateSpaceBottomSheet(model = model)
              }
            },
          ),
        )
      },
    )
  }
}

@Composable
private fun BottomSheetScope<Unit>.CreateSpaceBottomSheet(
  model: SpacePopoverViewModel,
) {
  var name by remember { mutableStateOf("") }

  BottomSheetScaffold(
    title = "새 스페이스 생성",
  ) {
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
    )

    Row(
      horizontalArrangement = Arrangement.spacedBy(12.dp),
      modifier = Modifier.fillMaxWidth(),
    ) {
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
          if (model.createSite(name)) {
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
  Box(
    modifier = Modifier
      .fillMaxWidth()
      .height(TopBarDefaults.ButtonSize),
  ) {
    Box(
      contentAlignment = Alignment.CenterStart,
      modifier = Modifier
        .fillMaxSize()
        .padding(start = 46.dp, end = 16.dp),
    ) {
      Text(
        site.name,
        style = AppTheme.typography.label,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }

    PopoverTransitionElement(
      collapsedFrame = PopoverTransitionFrame(
        left = 9.dp,
        top = (TopBarDefaults.ButtonSize - 26.dp) / 2,
        width = 26.dp,
        height = 26.dp,
      ),
      expandedFrame = PopoverTransitionFrame(
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

  Img(
    image = logo,
    modifier = Modifier.size(size).clip(logoShape),
  )
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
    Icon(
      icon = icon,
      modifier = Modifier.size(18.dp),
      tint = AppTheme.colors.textPrimary,
    )
    Spacer(Modifier.width(12.dp))
    Text(
      label,
      style = AppTheme.typography.action,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}
