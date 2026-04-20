import Testing
import Foundation
@testable import SighFarCore

@Suite("TerminalCapabilities")
struct TerminalCapabilitiesTests {

    // MARK: – NO_COLOR suppresses ANSI

    @Test func noColorDisablesANSI() {
        let caps = TerminalCapabilities(environment: [
            "NO_COLOR": "",
            "TERM": "xterm-256color",
        ])
        #expect(!caps.supportsANSI)
    }

    @Test func noColorReturnsPlainText() {
        let caps = TerminalCapabilities(environment: ["NO_COLOR": "1"])
        let result = caps.styled("hello", code: "31")
        #expect(result == "hello")
    }

    // MARK: – Unix TERM detection

    @Test func xtermEnablesANSI() {
        let caps = TerminalCapabilities(environment: ["TERM": "xterm-256color"])
        #expect(caps.supportsANSI)
    }

    @Test func dumbTermDisablesANSI() {
        let caps = TerminalCapabilities(environment: ["TERM": "dumb"])
        #expect(!caps.supportsANSI)
    }

    @Test func emptyTermDisablesANSI() {
        let caps = TerminalCapabilities(environment: ["TERM": ""])
        #expect(!caps.supportsANSI)
    }

    @Test func missingTermDisablesANSI() {
        let caps = TerminalCapabilities(environment: [:])
        #expect(!caps.supportsANSI)
    }

    // MARK: – styled() output

    @Test func styledWrapsWithEscapeCodes() {
        let caps = TerminalCapabilities(environment: ["TERM": "xterm-256color"])
        let result = caps.styled("text", code: "31")
        #expect(result == "\u{001B}[31mtext\u{001B}[0m")
    }

    @Test func styledPassthroughWhenNoANSI() {
        let caps = TerminalCapabilities(environment: ["NO_COLOR": "1"])
        let result = caps.styled("text", code: "31")
        #expect(result == "text")
    }

    // MARK: – clearSequence

    @Test func clearSequencePresentWhenANSISupported() {
        let caps = TerminalCapabilities(environment: ["TERM": "xterm-256color"])
        #expect(!caps.clearSequence.isEmpty)
    }

    @Test func clearSequenceEmptyWhenNoANSI() {
        let caps = TerminalCapabilities(environment: ["NO_COLOR": "1"])
        #expect(caps.clearSequence.isEmpty)
    }

    // MARK: – Windows fallback simulation (non-Windows builds use TERM path)

    #if !os(Windows)
    @Test func noTermAndNoWindowsEnvDisablesANSI() {
        // Simulate a Windows-like CI env on a non-Windows host (no TERM, no WT_SESSION etc.)
        let caps = TerminalCapabilities(environment: [:])
        #expect(!caps.supportsANSI)
    }
    #endif
}
