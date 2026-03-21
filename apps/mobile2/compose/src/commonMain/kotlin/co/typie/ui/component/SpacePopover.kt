package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.graphql.QueryState
import co.typie.graphql.SpacePopover_Query
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.rememberQuery
import co.typie.icons.Lucide
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPosition
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.PopoverTransitionElement
import co.typie.ui.component.popover.PopoverTransitionFrame
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme

val SpacePopoverLeadingKey = Any()

@Composable
fun SpacePopover() {
  val query = rememberQuery(SpacePopover_Query())
  val me = (query.state as? QueryState.Success)?.data?.me

  val logo = me?.sites?.firstOrNull()?.logo?.img_image
  val otherSites = me?.sites?.drop(1) ?: emptyList()

  Popover(
    position = PopoverPosition.BottomLeft,
    collapsedCornerRadius = 14.dp,
    anchor = { SpacePopoverAnchor(logo = logo) },
    pane = { SpacePopoverPane(logo = logo, otherSites = otherSites) },
  )
}

@Composable
private fun SpacePopoverAnchor(logo: Img_image?) {
  val outerShape = SquircleShape(14.dp)
  val logoShape = RoundedCornerShape(6.dp)

  Box(
    contentAlignment = Alignment.Center,
    modifier = Modifier.size(TopBarDefaults.ButtonSize)
      .then(TopBarDefaults.controlShadowModifier(outerShape)).clip(outerShape)
      .background(TopBarDefaults.controlBackgroundColor(), outerShape)
      .border(1.dp, TopBarDefaults.controlBorderColor(), outerShape),
  ) {
    if (logo != null) {
      Img(
        image = logo,
        size = 26.dp,
        modifier = Modifier.clip(logoShape),
      )
    } else {
      Icon(
        icon = Lucide.FolderOpen,
        modifier = Modifier.size(18.dp),
        tint = AppTheme.colors.textDefault,
      )
    }
  }
}

@Composable
private fun PopoverScope.SpacePopoverPane(
  logo: Img_image?,
  otherSites: List<SpacePopover_Query.Site>,
) {
  val panePadding = PopoverDefaults.PanePadding

  Column(
    modifier = Modifier.padding(panePadding),
  ) {
    SpacePopoverHeader(logo = logo)

    Spacer(Modifier.height(4.dp))

    PopoverList(
      items = listOf(
        PopoverListItem(
          content = { SpacePopoverItem(icon = Lucide.Settings, label = "스페이스 설정") },
          onSelected = { close() },
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
        .background(AppTheme.colors.borderElevated),
    )

    Spacer(Modifier.height(12.dp))

    Text(
      "다른 스페이스",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textFaint,
      modifier = Modifier.padding(horizontal = 8.dp),
    )

    Spacer(Modifier.height(8.dp))

    PopoverList(
      items = buildList {
        for (site in otherSites) {
          add(
            PopoverListItem(
              content = { SpacePopoverSiteItem(logo = site.logo.img_image, name = site.name) },
              onSelected = { close() },
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
private fun SpacePopoverHeader(logo: Img_image?) {
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
        "내 스페이스",
        style = AppTheme.typography.title,
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
      SpacePopoverLogo(logo = logo, size = 26.dp)
    }
  }
}

@Composable
private fun SpacePopoverLogo(logo: Img_image?, size: androidx.compose.ui.unit.Dp) {
  val logoShape = RoundedCornerShape(6.dp)

  if (logo != null) {
    Img(
      image = logo,
      size = size,
      modifier = Modifier.clip(logoShape),
    )
  } else {
    Box(
      contentAlignment = Alignment.Center,
      modifier = Modifier.size(size),
    ) {
      Icon(
        icon = Lucide.FolderOpen,
        modifier = Modifier.size(size * 0.75f),
        tint = AppTheme.colors.textDefault,
      )
    }
  }
}

@Composable
private fun SpacePopoverSiteItem(logo: Img_image?, name: String) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = Modifier.height(48.dp).padding(horizontal = 16.dp),
  ) {
    SpacePopoverLogo(logo = logo, size = 28.dp)
    Spacer(Modifier.width(12.dp))
    Text(
      name,
      style = AppTheme.typography.action,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun SpacePopoverItem(icon: co.typie.ui.icon.IconData, label: String) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
  ) {
    Icon(
      icon = icon,
      modifier = Modifier.size(18.dp),
      tint = AppTheme.colors.textDefault,
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
