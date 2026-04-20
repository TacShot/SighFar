import Foundation

/// Resolves the SighFar application-support directory in a platform-safe way.
///
/// | Platform       | Directory                          |
/// |----------------|------------------------------------|
/// | macOS / Linux  | `~/.sighfar`                       |
/// | Windows        | `%APPDATA%\SighFar`                |
///
/// The `environment` property is injectable so tests can verify behaviour
/// without touching the real file system.
struct PlatformPaths {
    let environment: [String: String]

    init(environment: [String: String] = ProcessInfo.processInfo.environment) {
        self.environment = environment
    }

    /// The directory used to store the encrypted history file and its key.
    var sighfarDirectory: URL {
        #if os(Windows)
        if let appdata = environment["APPDATA"], !appdata.isEmpty {
            return URL(fileURLWithPath: appdata)
                .appendingPathComponent("SighFar", isDirectory: true)
        }
        // Fallback when APPDATA is unset (e.g. in CI).
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("SighFar", isDirectory: true)
        #else
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".sighfar", isDirectory: true)
        #endif
    }
}
