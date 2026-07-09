package co.typie.editor.sync

import app.cash.sqldelight.db.SqlDriver
import app.cash.sqldelight.driver.jdbc.sqlite.JdbcSqliteDriver
import co.typie.sync.db.SyncDatabase
import java.io.File

actual fun createSyncDatabaseDriver(): SqlDriver {
  val dir = File(System.getProperty("user.home"), ".local/share/typie")
  dir.mkdirs()
  return createDesktopSyncDriver(File(dir, "typie-sync.db").absolutePath)
}

internal fun createDesktopSyncDriver(dbPath: String): SqlDriver =
  JdbcSqliteDriver(
    url = "jdbc:sqlite:$dbPath",
    properties = java.util.Properties(),
    schema = SyncDatabase.Schema,
  )
