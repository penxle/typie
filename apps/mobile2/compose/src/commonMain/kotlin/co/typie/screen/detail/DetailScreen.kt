package co.typie.screen.detail

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarTitle
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject

private val DetailTrailingKey = Any()

@Composable
fun DetailScreen(id: String) {
  val nav = Nav.current
  val toast = koinInject<Toast>()

  ProvideTopBar(
    center = {
      TopBarTitle(id, subtitle = "Detail Screen", icon = Lucide.FolderOpen)
    },
    trailingKey = DetailTrailingKey,
    trailing = {
      TopBarButton(
        icon = Lucide.Info,
        onClick = { toast.show(ToastType.Notification, "Info clicked") })
    },
  )

  Screen { contentPadding ->
    Column(
      Modifier.fillMaxSize().padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text("Detail: $id", style = AppTheme.typography.body)
      Text(
        "Go to next detail",
        color = AppTheme.colors.brand,
        style = AppTheme.typography.body,
        modifier = Modifier.clickable {
          nav.navigate(Route.Detail(id + 1))
        })
    }
  }
}

