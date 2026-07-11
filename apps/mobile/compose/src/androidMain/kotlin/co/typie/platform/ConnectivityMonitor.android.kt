package co.typie.platform

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow

actual fun connectivityAvailabilityFlow(): Flow<Boolean> = callbackFlow {
  val manager =
    PlatformModule.context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
  var defaultNetwork: Network? = null
  val callback =
    object : ConnectivityManager.NetworkCallback() {
      override fun onAvailable(network: Network) {
        defaultNetwork = network
      }

      override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
        if (network == defaultNetwork) {
          trySend(capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED))
        }
      }

      override fun onLost(network: Network) {
        if (network == defaultNetwork) {
          defaultNetwork = null
          trySend(false)
        }
      }
    }
  manager.registerDefaultNetworkCallback(callback)
  awaitClose { manager.unregisterNetworkCallback(callback) }
}
