package co.typie.platform

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow

actual fun connectivityRestoredFlow(): Flow<Unit> = callbackFlow {
  val manager =
    PlatformModule.context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
  val callback =
    object : ConnectivityManager.NetworkCallback() {
      override fun onAvailable(network: Network) {
        trySend(Unit)
      }
    }
  manager.registerDefaultNetworkCallback(callback)
  awaitClose { manager.unregisterNetworkCallback(callback) }
}
