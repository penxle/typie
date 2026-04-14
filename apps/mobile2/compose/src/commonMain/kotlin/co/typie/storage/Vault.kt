package co.typie.storage

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import co.typie.domain.auth.AuthTokens
import co.typie.platform.PlatformModule
import eu.anifantakis.lib.ksafe.invoke

internal inline fun <reified T> vault(key: String, defaultValue: T): PersistentState<T> {
  val delegate = PlatformModule.ksafeVault.invoke(defaultValue, key)
  val holder =
    object {
      var v: T by delegate
    }
  val initial = holder.v
  return PersistentState(initial) { holder.v = it }
}

object Vault {
  var authTokens by vault<AuthTokens?>("auth_tokens", null)
}
