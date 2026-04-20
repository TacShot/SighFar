import Testing
import Foundation
@testable import SighFarCore

@Suite("PlatformPaths")
struct PlatformPathsTests {

    // MARK: – macOS / Linux path

    @Test func defaultPathUsesHomeDirectory() {
        // Inject a clean env with no APPDATA so we always exercise the
        // non-Windows branch regardless of which OS the test runs on.
        let paths = PlatformPaths(environment: [:])
        let dir = paths.sighfarDirectory

        #if os(Windows)
        // Without APPDATA the Windows fallback is ~/SighFar.
        #expect(dir.lastPathComponent == "SighFar")
        #else
        #expect(dir.lastPathComponent == ".sighfar")
        #endif
    }

    #if os(Windows)
    @Test func windowsUsesAPPDATA() {
        let paths = PlatformPaths(environment: ["APPDATA": "C:\\Users\\test\\AppData\\Roaming"])
        let dir = paths.sighfarDirectory
        #expect(dir.lastPathComponent == "SighFar")
        #expect(dir.path.hasPrefix("C:"))
    }

    @Test func windowsFallbackWhenAPPDATAMissing() {
        let paths = PlatformPaths(environment: [:])
        let dir = paths.sighfarDirectory
        #expect(dir.lastPathComponent == "SighFar")
    }
    #else
    @Test func unixPathEndsWithSighfar() {
        let paths = PlatformPaths(environment: ["TERM": "xterm-256color"])
        let dir = paths.sighfarDirectory
        #expect(dir.lastPathComponent == ".sighfar")
    }
    #endif
}
