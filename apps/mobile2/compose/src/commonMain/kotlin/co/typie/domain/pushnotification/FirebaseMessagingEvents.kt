package co.typie.domain.pushnotification

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow

internal object FirebaseMessagingEvents {
  val message: MutableSharedFlow<PushNotificationMessage> =
    MutableSharedFlow(
      replay = 0,
      extraBufferCapacity = 8,
      onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )

  val tokenRefresh: MutableSharedFlow<String> =
    MutableSharedFlow(
      replay = 0,
      extraBufferCapacity = 8,
      onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
}
