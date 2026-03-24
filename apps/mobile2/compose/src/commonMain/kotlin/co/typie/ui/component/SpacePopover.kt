package co.typie.ui.component

import androidx.compose.foundation.background
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.QueryState
import co.typie.graphql.SpacePopover_Query
import co.typie.graphql.fragment.Img_image
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.service.SiteService
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
class SpacePopoverViewModel : GraphQLViewModel() {
  val query = watchQuery { SpacePopover_Query() }
}

@Composable
fun SpacePopover() {
  val model = koinViewModel<SpacePopoverViewModel>()
  val siteService = koinInject<SiteService>()

  Skeleton(enabled = model.query.state !is QueryState.Success) {
    when (val state = model.query.state) {
      is QueryState.Success -> {
        val currentSite = state.data.me.sites.first { it.id == siteService.siteId }
        val otherSites = state.data.me.sites.filter { it.id != currentSite.id }

        Popover(
          placement = PopoverPlacement.BelowStart,
          screenPadding = SpacePopoverScreenPadding,
          collapsedCornerRadius = 14.dp,
          anchor = { SpacePopoverAnchor(currentSite) },
          pane = { SpacePopoverPane(currentSite, otherSites) },
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
  currentSite: SpacePopover_Query.Site,
  otherSites: List<SpacePopover_Query.Site>,
) {
  val nav = Nav.current
  val scope = rememberCoroutineScope()
  val siteService = koinInject<SiteService>()
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
          onSelected = { close() },
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
            onSelected = { close() },
          ),
        )
      },
    )
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
