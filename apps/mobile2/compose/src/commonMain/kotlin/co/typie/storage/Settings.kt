package co.typie.storage

import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeWriteMode
import eu.anifantakis.lib.ksafe.invoke
import org.koin.core.annotation.Named
import org.koin.core.annotation.Single
import kotlin.properties.ReadWriteProperty

@Single
class Prefs(@Named("ksafe.prefs") val prefs: KSafe) {
  inline operator fun <reified T> invoke(
    defaultValue: T,
    key: String? = null
  ): ReadWriteProperty<Any?, T> =
    prefs.invoke(defaultValue, key, mode = KSafeWriteMode.Plain)
}

@Single
class Vault(@Named("ksafe.vault") val vault: KSafe) {
  inline operator fun <reified T> invoke(
    defaultValue: T,
    key: String? = null
  ): ReadWriteProperty<Any?, T> =
    vault.invoke(defaultValue, key)
}
