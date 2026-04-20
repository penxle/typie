package co.typie.domain.pushnotification

import android.Manifest
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import kotlinx.coroutines.CompletableDeferred

object NotificationPermissionLauncher {
  private var launcher: ActivityResultLauncher<String>? = null
  private var pending: CompletableDeferred<Boolean>? = null

  fun register(activity: ComponentActivity) {
    launcher =
      activity.registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        pending?.complete(granted)
        pending = null
      }
  }

  suspend fun request(): Boolean {
    val l = launcher ?: return false
    val d = CompletableDeferred<Boolean>().also { pending = it }
    l.launch(Manifest.permission.POST_NOTIFICATIONS)
    return d.await()
  }
}
