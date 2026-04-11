package co.typie.storage

import co.typie.auth.AuthTokens
import co.typie.platform.PlatformModule
import eu.anifantakis.lib.ksafe.invoke

internal inline fun <reified T> vault(key: String, defaultValue: T): PersistentStateFlow<T> {
  val delegate = PlatformModule.ksafeVault.invoke(defaultValue, key)
  val holder =
    object {
      var v: T by delegate
    }
  val initial = holder.v
  return PersistentStateFlow(initial) { holder.v = it }
}

object Vault {
  val authTokens = vault<AuthTokens?>("auth_tokens", null)
}
