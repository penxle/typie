package co.typie.domain.pushnotification

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import co.typie.platform.PlatformModule
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.tasks.await

actual object FirebaseMessaging {
  actual val onMessage: Flow<PushNotificationMessage> =
    FirebaseMessagingEvents.message.asSharedFlow()

  actual val onTokenRefresh: Flow<String> = FirebaseMessagingEvents.tokenRefresh.asSharedFlow()

  actual suspend fun requestPermission(): Boolean {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return true
    val granted =
      ContextCompat.checkSelfPermission(
        PlatformModule.context,
        Manifest.permission.POST_NOTIFICATIONS,
      ) == PackageManager.PERMISSION_GRANTED
    if (granted) return true
    return NotificationPermissionLauncher.request()
  }

  actual suspend fun token(): String? =
    try {
      com.google.firebase.messaging.FirebaseMessaging.getInstance().token.await()
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      null
    }

  actual suspend fun deleteToken() {
    try {
      com.google.firebase.messaging.FirebaseMessaging.getInstance().deleteToken().await()
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      // best-effort
    }
  }
}
