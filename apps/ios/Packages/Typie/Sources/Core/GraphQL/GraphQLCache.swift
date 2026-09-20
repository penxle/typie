import Apollo
import ApolloSQLite
import Foundation

enum GraphQLCache {
  static func makeStore() -> ApolloStore {
    guard let url = try? fileURL(),
      let cache = try? SQLiteNormalizedCache(fileURL: url, shouldVacuumOnClear: true)
    else {
      return ApolloStore()
    }
    return ApolloStore(cache: cache)
  }

  private static func fileURL() throws -> URL {
    let directory = try FileManager.default.url(
      for: .cachesDirectory, in: .userDomainMask, appropriateFor: nil, create: true
    ).appending(path: "GraphQL", directoryHint: .isDirectory)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    return directory.appending(path: "cache.sqlite")
  }
}
