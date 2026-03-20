package co.typie.screen.home

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.RandomNameQuery
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.clickable
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.koin.compose.viewmodel.koinViewModel
import org.koin.core.annotation.KoinViewModel

data class HomeState(
  val randomName: String? = null,
  val isLoading: Boolean = false,
)

@KoinViewModel
class HomeViewModel(private val apolloClient: ApolloClient) : ViewModel() {
  private val _state = MutableStateFlow(HomeState())
  val state: StateFlow<HomeState> = _state

  fun fetchRandomName() {
    viewModelScope.launch {
      _state.update { it.copy(isLoading = true) }
      val response = apolloClient.query(RandomNameQuery()).execute()
      _state.update { it.copy(randomName = response.data?.randomName, isLoading = false) }
    }
  }
}

@Composable
fun HomeScreen() {
  val nav = Nav.current
  val viewModel = koinViewModel<HomeViewModel>()
  val state by viewModel.state.collectAsState()

  Screen {
    Column(Modifier.fillMaxSize().padding(16.dp)) {
      Text("Home", style = TextStyle(fontSize = 20.sp))
      Text(
        if (state.isLoading) "Loading..."
        else "Random Name: ${state.randomName ?: "tap to fetch"}",
        style = TextStyle(color = AppTheme.colors.accentInfo, fontSize = 16.sp),
        modifier = Modifier.padding(top = 12.dp).clickable { viewModel.fetchRandomName() },
      )
      Text(
        "Go to Detail",
        style = TextStyle(color = AppTheme.colors.accentInfo, fontSize = 16.sp),
        modifier = Modifier.padding(top = 12.dp).clickable { nav.navigate(Route.Detail("123")) },
      )
    }
  }
}