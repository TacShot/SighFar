import Foundation

struct TerminalUI {
    private let line = String(repeating: "=", count: 68)

    func renderHeader() {
        clearScreen()
        print("\u{001B}[31m\(line)\u{001B}[0m")
        print("\u{001B}[31;1m  SmileFar // offline cipher workbench\u{001B}[0m")
        print("\u{001B}[90m  retro shell prototype inspired by SmileOS aesthetics\u{001B}[0m")
        print("\u{001B}[31m\(line)\u{001B}[0m")
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
        print("\u{001B}[31;1m[ \(title) ]\u{001B}[0m")
        print(body)
        print("")
    }

    func prompt(_ text: String) -> String {
        print(text, terminator: " ")
        fflush(stdout)
        return readLine() ?? ""
    }

    func pause() {
        _ = prompt("Press return to continue...")
    }

    func clearScreen() {
        print("\u{001B}[2J\u{001B}[H", terminator: "")
    }
}
