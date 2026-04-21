package co.typie.domain.pushnotification

data class PushNotificationMessage(
  val title: String?,
  val body: String?,
  val data: Map<String, String>,
)
