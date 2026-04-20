import Foundation

/// Detects at runtime whether the current terminal supports ANSI color sequences.
///
/// Respects the cross-platform `NO_COLOR` convention (https://no-color.org).
/// On Windows, checks for Windows Terminal or any VT-capable host.
/// On Unix/macOS, checks the `TERM` environment variable.
struct TerminalCapabilities {
    let environment: [String: String]

    init(environment: [String: String] = ProcessInfo.processInfo.environment) {
        self.environment = environment
    }

    /// Whether the terminal can render ANSI escape sequences.
    var supportsANSI: Bool {
        // NO_COLOR (any value) means "disable colour" regardless of platform.
        if environment["NO_COLOR"] != nil { return false }

        #if os(Windows)
        // Windows Terminal sets WT_SESSION; ConEmu/similar set TERM_PROGRAM or COLORTERM.
        return environment["WT_SESSION"] != nil
            || environment["TERM_PROGRAM"] != nil
            || environment["COLORTERM"] != nil
        #else
        let term = environment["TERM"] ?? ""
        return term != "dumb" && !term.isEmpty
        #endif
    }

    /// Wraps `text` in an ANSI SGR sequence when the terminal supports it;
    /// otherwise returns the text unmodified.
    func styled(_ text: String, code: String) -> String {
        guard supportsANSI else { return text }
        return "\u{001B}[\(code)m\(text)\u{001B}[0m"
    }

    /// The ANSI clear-screen + cursor-home sequence, or an empty string on
    /// terminals that do not support ANSI.
    var clearSequence: String {
        supportsANSI ? "\u{001B}[2J\u{001B}[H" : ""
    }
}
