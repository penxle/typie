package co.typie.shell

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import co.touchlab.kermit.Logger
import co.typie.graphql.Apollo
import co.typie.graphql.MainShell_SiteUpdateStream_Subscription
import co.typie.storage.Preference
import com.apollographql.apollo.annotations.ApolloExperimental

@OptIn(ApolloExperimental::class)
@Composable
internal fun SiteUpdateStreamEffect() {
  val siteId = Preference.siteId.value

  LaunchedEffect(siteId) {
    if (siteId.isNullOrBlank()) {
      return@LaunchedEffect
    }

    Apollo.subscription(MainShell_SiteUpdateStream_Subscription(siteId = siteId))
      .retryOnError(true)
      .toFlow()
      .collect { response ->
        response.exception?.let { error -> Logger.e(error) { "siteUpdateStream failed" } }

        response.errors
          ?.takeIf { it.isNotEmpty() }
          ?.let { errors ->
            Logger.e { "siteUpdateStream graphql errors=${errors.joinToString { it.message }}" }
          }
      }
  }
}
