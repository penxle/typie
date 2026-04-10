package co.typie.screen.home.home_search

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.ime
import co.typie.ext.plus
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.screen.home.home.HomeSearchFieldDefaults
import co.typie.screen.home.home.resolveHomeSearchPlaceholder
import co.typie.ui.component.Screen
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

private val SearchScreenTopFadeHeight = 24.dp
private val SearchScreenHeaderHeight = HomeSearchFieldDefaults.Height + 4.dp

@Composable
fun HomeSearchScreen() {
  val nav = Nav.current
  val model = viewModel { SearchViewModel() }
  val scrollState = rememberScrollState("home-search")

  ProvideTopBar(leading = { TopBarBackButton(onClick = { nav.pop() }) })

  Screen(
    background = AppTheme.colors.surfaceBase,
    responsive = true,
    contentPadding = PaddingValues(0.dp),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
      val bottomInset =
        maxOf(
          WindowInsets.ime.asPaddingValues().calculateBottomPadding(),
          WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding(),
        )

      Box(Modifier.fillMaxSize()) {
        Column(
          modifier =
            Modifier.fillMaxSize()
              .verticalScroll(scrollState)
              .padding(contentPadding + PaddingValues(bottom = bottomInset + 12.dp))
        ) {
          SearchContent(
            searchViewModel = model,
            headerHeight = SearchScreenHeaderHeight,
            onDocumentClick = { slug, query ->
              model.saveRecentSearch(query)
              nav.navigate(Route.Editor(slug))
            },
            onFolderClick = { entityId, query ->
              model.saveRecentSearch(query)
              nav.navigate(Route.Folder(entityId))
            },
          )
        }

        SearchHeaderOverlay(
          modifier =
            Modifier.align(Alignment.TopCenter)
              .fillMaxWidth()
              .padding(top = contentPadding.calculateTopPadding())
        ) {
          SearchHeader(
            animateOnEnter = model.shouldAnimateHeaderOnEnter,
            placeholder = resolveHomeSearchPlaceholder(model.siteQuery.data.site.name),
            query = model.query,
            onQueryChange = { model.updateQuery(it) },
            onSubmit = { model.submitQuery() },
            onEnterAnimationConsumed = { model.onHeaderEnterAnimationConsumed() },
          )
        }
      }
    },
  )
}

@Composable
private fun SearchHeaderOverlay(modifier: Modifier = Modifier, header: @Composable () -> Unit) {
  Box(modifier = modifier) {
    Column(modifier = Modifier.fillMaxWidth().background(AppTheme.colors.surfaceBase)) { header() }

    SearchTopFade(
      modifier =
        Modifier.align(Alignment.BottomCenter).offset(y = SearchScreenTopFadeHeight).fillMaxWidth()
    )
  }
}

@Composable
private fun SearchTopFade(modifier: Modifier = Modifier) {
  Box(
    modifier =
      modifier
        .height(SearchScreenTopFadeHeight)
        .background(
          Brush.verticalGradient(
            colors =
              listOf(AppTheme.colors.surfaceBase, AppTheme.colors.surfaceBase.copy(alpha = 0f))
          )
        )
  )
}
