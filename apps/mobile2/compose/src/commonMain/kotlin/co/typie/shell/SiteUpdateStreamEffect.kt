package co.typie.shell

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import co.touchlab.kermit.Logger
import co.typie.graphql.MainShell_SiteUpdateStream_Subscription
import co.typie.service.SiteService
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.annotations.ApolloExperimental
import org.koin.compose.koinInject

@OptIn(ApolloExperimental::class)
@Composable
internal fun SiteUpdateStreamEffect() {
  val apolloClient = koinInject<ApolloClient>()
  val siteService = koinInject<SiteService>()
  val siteId = siteService.siteId

  LaunchedEffect(siteId) {
    if (siteId.isBlank()) {
      return@LaunchedEffect
    }

    apolloClient.subscription(MainShell_SiteUpdateStream_Subscription(siteId = siteId))
      .retryOnError(true)
      .toFlow()
      .collect { response ->
        response.exception?.let { error ->
          Logger.e(error) { "siteUpdateStream failed" }
        }

        response.errors?.takeIf { it.isNotEmpty() }?.let { errors ->
          Logger.e { "siteUpdateStream graphql errors=${errors.joinToString { it.message }}" }
        }
      }
  }
}
