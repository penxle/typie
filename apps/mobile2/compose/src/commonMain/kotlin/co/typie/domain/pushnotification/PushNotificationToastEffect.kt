package co.typie.domain.pushnotification

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import co.typie.ui.component.toast.LocalToast
import kotlin.time.Duration.Companion.seconds

@Composable
fun PushNotificationToastEffect() {
  val toast = LocalToast.current
  LaunchedEffect(Unit) {
    FirebaseMessaging.onMessage.collect { message ->
      val title = message.title ?: return@collect
      toast.notification(title, duration = 10.seconds)
    }
  }
}
