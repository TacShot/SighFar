import Foundation

struct TerminalUI {
    private let line = String(repeating: "=", count: 68)
    private let caps = TerminalCapabilities()

    func renderHeader() {
        clearScreen()
        print(caps.styled(line, code: "31"))
        print(caps.styled("  SmileFar // offline cipher workbench", code: "31;1"))
        print(caps.styled("  retro shell prototype inspired by SmileOS aesthetics", code: "90"))
        print(caps.styled(line, code: "31"))
        print("")
        print("  [1] Encode message")
        print("  [2] Decode message")
        print("  [3] View encrypted history")
        print("  [4] Settings")
        print("  [5] Roadmap")
        print("  [0] Quit")
        print("")
    }

    func printPanel(title: String, body: String) {
        print(caps.styled("[ \(title) ]", code: "31;1"))
        print(body)
        print("")
    }

    func prompt(_ text: String) -> String {
        // The C standard guarantees stdout is flushed before stdin is read on
        // interactive terminals, so an explicit fflush is not required here.
        // On non-interactive terminals or when output is redirected (e.g. in
        // CI), output is captured rather than displayed, so flush ordering is
        // irrelevant in those scenarios.
        print(text, terminator: " ")
        return readLine() ?? ""
    }

    func pause() {
        _ = prompt("Press return to continue...")
    }

    func clearScreen() {
        let seq = caps.clearSequence
        if !seq.isEmpty {
            print(seq, terminator: "")
        }
    }
}

