package co.typie.editor.sync

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.android.AndroidSqliteDriver
import co.typie.platform.PlatformModule
import co.typie.sync.db.SyncDatabase

actual fun createSyncDatabaseDriver(): SqlDriver =
  AndroidSqliteDriver(SyncDatabase.Schema, PlatformModule.context, "typie-sync.db")
