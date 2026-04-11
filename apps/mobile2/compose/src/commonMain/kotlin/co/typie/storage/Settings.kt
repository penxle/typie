package co.typie.storage

import co.typie.platform.PlatformModule
import eu.anifantakis.lib.ksafe.KSafeWriteMode
import eu.anifantakis.lib.ksafe.invoke
import kotlin.properties.ReadWriteProperty

inline fun <reified T> prefs(key: String, defaultValue: T): ReadWriteProperty<Any?, T> =
  PlatformModule.ksafePrefs.invoke(defaultValue, key, mode = KSafeWriteMode.Plain)

inline fun <reified T> vault(key: String, defaultValue: T): ReadWriteProperty<Any?, T> =
  PlatformModule.ksafeVault.invoke(defaultValue, key)
