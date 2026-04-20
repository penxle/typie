package co.typie.shell

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.Img_image
import co.typie.storage.Preference
import co.typie.ui.component.Img
import co.typie.ui.component.drawer.LocalDrawer
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppShapes
import kotlinx.coroutines.launch

val MainDrawerTriggerLeadingKey: Any = Any()

private val TriggerHeight = 44.dp
private val TriggerMaxWidth = 200.dp
private val TriggerLogoSize = 24.dp

@Composable
fun MainDrawerTrigger() {
  val model = viewModel { MainDrawerViewModel() }
  val drawer = LocalDrawer.current
  val scope = rememberCoroutineScope()

  InteractionScope {
    Skeleton(enabled = model.query.state !is QueryState.Success) {
      val data = model.query.data
      val availableSiteIds = data.me.sites.map { it.id }
      val selection =
        resolveMainDrawerSelection(
          selectedSiteId = Preference.siteId.orEmpty(),
          availableSiteIds = availableSiteIds,
        )
      val currentSite = data.me.sites.first { it.id == selection.currentSiteId }

      TriggerContent(logo = currentSite.logo.img_image) { scope.launch { drawer.open() } }
    }
  }
}

@Composable
private fun TriggerContent(logo: Img_image, onClick: suspend () -> Unit) {
  val shape = AppShapes.squircle(AppShapes.md)
  val logoShape = AppShapes.rounded(AppShapes.sm)

  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier =
      Modifier.height(TriggerHeight)
        .widthIn(max = TriggerMaxWidth)
        .pressScale()
        .clip(shape)
        .background(TopBarDefaults.controlBackgroundColor(), shape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), shape)
        .clickable(onClick = onClick)
        .padding(horizontal = 10.dp),
  ) {
    Img(image = logo, modifier = Modifier.size(TriggerLogoSize).clip(logoShape))
  }
}
