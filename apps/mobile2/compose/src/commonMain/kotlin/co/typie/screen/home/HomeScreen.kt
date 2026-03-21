package co.typie.screen.home

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.GraphQLContent
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.QueryState
import co.typie.graphql.rememberQuery
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun HomeScreen() {
  val nav = Nav.current
  val query = rememberQuery(HomeScreen_Query())
  val scrollState = rememberScrollState()

  val data = (query.state as? QueryState.Success)?.data

  ProvideTopBar(
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("홈", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen { contentPadding ->
    GraphQLContent(query) { data ->
      Column(
        Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding)
          .padding(horizontal = 16.dp).navigationBarsPadding()
      ) {
        Text("홈", style = AppTheme.typography.display)
        Text(
          "Go to Detail",
          color = AppTheme.colors.accentInfo,
          modifier = Modifier.padding(top = 12.dp).clickable { nav.navigate(Route.Detail("123")) },
        )
        for (i in 1..30) {
          Text(
            "Item $i",
            modifier = Modifier.padding(top = 16.dp),
          )
        }
      }
    }
  }
}
