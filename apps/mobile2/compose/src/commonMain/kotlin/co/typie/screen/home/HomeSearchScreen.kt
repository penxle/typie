package co.typie.screen.home

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.navigation.Nav
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.Screen
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun HomeSearchScreen() {
  val nav = Nav.current
  val model = koinViewModel<SearchViewModel>()
  val scrollState = rememberScrollState("home-search")
  val bottomBarState = LocalBottomBarState.current

  LaunchedEffect(Unit) {
    bottomBarState.visible = false
  }

  ProvideTopBar(
    leadingKey = SearchTopBarKey,
    leading = { TopBarBackButton(onClick = { nav.pop() }) },
  )

  Screen(
    background = AppTheme.colors.surfaceBase,
    responsive = true,
    imeAware = true,
    contentPadding = PaddingValues(0.dp),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
      Column(
        Modifier
          .fillMaxSize()
          .padding(contentPadding)
      ) {
        SearchHeader(
          animateOnEnter = model.shouldAnimateHeaderOnEnter,
          query = model.query,
          onQueryChange = { model.updateQuery(it) },
          onSubmit = { model.submitQuery() },
          onEnterAnimationConsumed = { model.onHeaderEnterAnimationConsumed() },
        )

        SearchContent(
          modifier = Modifier.weight(1f),
          searchViewModel = model,
          contentPadding = PaddingValues(0.dp),
          scrollState = scrollState,
        )
      }
    },
  )
}
