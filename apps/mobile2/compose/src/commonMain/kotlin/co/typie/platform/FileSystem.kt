package co.typie.platform

enum class FileSystemSaveLocation { Gallery, Files }
enum class FileSystemSaveResult { Success, PermissionDenied, Error }

interface FileSystem {
  suspend fun save(bytes: ByteArray, name: String, location: FileSystemSaveLocation): FileSystemSaveResult
}
