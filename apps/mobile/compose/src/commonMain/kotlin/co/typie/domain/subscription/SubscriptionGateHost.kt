package co.typie.domain.subscription

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.sheet.LocalSheet

@Composable
fun SubscriptionGateHost() {
  val sheet = LocalSheet.current
  val nav = Nav.current

  LaunchedEffect(Unit) {
    SubscriptionService.gateRequests.collect {
      val result = sheet.present { SubscribeSheet() }
      SubscriptionService.drainGateRequests()
      if (result is SubscribeSheetResult.Subscribe) {
        nav.navigate(Route.EnrollPlan)
      }
    }
  }
}
