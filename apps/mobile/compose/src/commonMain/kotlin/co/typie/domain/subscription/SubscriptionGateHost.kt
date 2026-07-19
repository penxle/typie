package co.typie.domain.subscription

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import co.typie.ui.component.sheet.LocalSheet

@Composable
fun SubscriptionGateHost() {
  val sheet = LocalSheet.current

  LaunchedEffect(Unit) {
    SubscriptionService.gateRequests.collect {
      sheet.presentSubscribeSheet()
      SubscriptionService.drainGateRequests()
    }
  }
}
