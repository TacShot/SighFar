import Foundation

package struct SighFarApp {
    /// Maximum message size (bytes) that is processed without warning.
    /// Inputs beyond this threshold prompt the user to confirm before encoding/decoding
    /// to prevent accidental large payloads from stalling the terminal.
    static let messageSizeLimitBytes = 1_048_576 // 1 MB

    private let ui = TerminalUI()
    private let pipeline = CipherPipeline()
    private let secureEnvelope = SecureEnvelope()
    private let historyStore = HistoryStore()
    private let parser = TechniqueParser()

    package init() {}

    package mutating func run() {
        var shouldContinue = true
        while shouldContinue {
            ui.renderHeader()
            let selection = ui.prompt("Choose a module:")

            switch selection {
            case "1":
                runEncodeFlow()
            case "2":
                runDecodeFlow()
            case "3":
                showHistory()
            case "4":
                showSettings()
            case "5":
                showRoadmap()
            case "0", "q", "quit":
                shouldContinue = false
            default:
                ui.printPanel(title: "Input", body: "Unknown option: \(selection)")
                ui.pause()
            }
        }
    }

    private func runEncodeFlow() {
        do {
            ui.clearScreen()
            ui.printPanel(
                title: "Encode",
                body: """
                Stack one or more techniques in order.
                Available: morse, caesar, vigenere, railfence, reverse
                Example: morse,caesar:4,reverse
                """
            )

            let message = ui.prompt("Message:")

            guard confirmLargeInput(message) else { return }

            let techniqueInput = ui.prompt("Technique chain:")
            let useSecureEnvelope = ui.prompt("Wrap in secure paired-key envelope? (y/N):")
            let techniques = try parser.parse(from: techniqueInput)
            let transformed = try pipeline.encode(message, using: techniques)

            var keyPair: SecureKeyPair?
            var securePayload: String?
            if useSecureEnvelope.lowercased().hasPrefix("y") {
                let passphrase = ui.prompt("Primary passphrase:")
                let pair = secureEnvelope.makeKeyPair(passphrase: passphrase)
                keyPair = pair
                securePayload = try secureEnvelope.seal(transformed, keyPair: pair)
            }

            let result = EncodedMessage(
                originalInput: message,
                transformedText: transformed,
                securePayload: securePayload,
                techniques: techniques,
                usedSecureEnvelope: keyPair != nil,
                keyPair: keyPair
            )

            try historyStore.append(historyEntry(for: result, operation: .encode))
            showEncodeResult(result)
        } catch {
            ui.printPanel(title: "Error", body: error.localizedDescription)
            ui.pause()
        }
    }

    private func runDecodeFlow() {
        do {
            ui.clearScreen()
            ui.printPanel(
                title: "Decode",
                body: """
                Enter the same technique chain used during encoding.
                If the message was wrapped in a secure envelope, provide both key parts.
                """
            )

            let secureWrapped = ui.prompt("Is this a secure payload? (y/N):")
            let rawInput: String

            if secureWrapped.lowercased().hasPrefix("y") {
                let payload = ui.prompt("Secure payload:")
                let passphrase = ui.prompt("Primary passphrase:")
                let companionCode = ui.prompt("Companion code:")
                rawInput = try secureEnvelope.open(
                    payload,
                    keyPair: SecureKeyPair(passphrase: passphrase, companionCode: companionCode)
                )
            } else {
                rawInput = ui.prompt("Cipher text:")
            }

            guard confirmLargeInput(rawInput) else { return }

            let techniqueInput = ui.prompt("Technique chain:")
            let techniques = try parser.parse(from: techniqueInput)
            let decoded = try pipeline.decode(rawInput, using: techniques)

            let result = EncodedMessage(
                originalInput: rawInput,
                transformedText: decoded,
                securePayload: nil,
                techniques: techniques,
                usedSecureEnvelope: secureWrapped.lowercased().hasPrefix("y"),
                keyPair: nil
            )

            try historyStore.append(historyEntry(for: result, operation: .decode))
            ui.printPanel(title: "Decoded", body: decoded)
            ui.pause()
        } catch {
            ui.printPanel(title: "Error", body: error.localizedDescription)
            ui.pause()
        }
    }

    private func showHistory() {
        do {
            ui.clearScreen()
            let entries = try historyStore.loadRecent(limit: 12)
            if entries.isEmpty {
                ui.printPanel(title: "History", body: "No entries yet. Encode or decode a message first.")
            } else {
                let body = entries.enumerated().map { index, entry in
                    let timestamp = ISO8601DateFormatter().string(from: entry.timestamp)
                    let techniques = entry.techniques.map(\.title).joined(separator: " -> ")
                    return """
                    \(index + 1). \(timestamp) [\(entry.operation.rawValue)]
                       in: \(entry.inputPreview)
                       out: \(entry.outputPreview)
                       chain: \(techniques)
                       secure: \(entry.usedSecureEnvelope ? "yes" : "no")
                    """
                }.joined(separator: "\n\n")
                ui.printPanel(title: "Encrypted History", body: body)
            }
            ui.pause()
        } catch {
            ui.printPanel(title: "Error", body: error.localizedDescription)
            ui.pause()
        }
    }

    private func showSettings() {
        ui.clearScreen()
        ui.printPanel(
            title: "Settings",
            body: """
            github oauth: planned
            update channel: planned
            file hiding / carrier mode: planned

            local encrypted history:
            \(historyStore.diagnostics())

            note:
            This prototype stores history in an encrypted local file and keeps the key in the app support directory.
            For stronger protection across platforms, move the key into platform keychains/credential vaults.
            """
        )
        ui.pause()
    }

    private func showRoadmap() {
        ui.clearScreen()
        ui.printPanel(
            title: "Roadmap",
            body: """
            Phase 1
            - offline terminal workbench
            - stacked ciphers + secure paired-key envelope
            - encrypted local history

            Phase 2
            - SmileOS-like GUI skin
            - drag-and-drop file carrier workflows
            - export/import key bundles

            Phase 3
            - github oauth in settings
            - signed release packaging per platform
            - updater behavior for macOS app replacement and version migration

            Platform note
            Swift gets us moving fast on macOS now. For Linux/Windows/Android/FreeBSD parity, keep the crypto core portable
            and consider a Rust or Flutter front-end layer once the workflow is locked in.
            """
        )
        ui.pause()
    }

    private func showEncodeResult(_ result: EncodedMessage) {
        var lines: [String] = []
        lines.append("Transformed text:")
        lines.append(result.transformedText)

        if let payload = result.securePayload, let keyPair = result.keyPair {
            lines.append("")
            lines.append("Secure payload:")
            lines.append(payload)
            lines.append("")
            lines.append("Share these separately:")
            lines.append("Primary passphrase: \(keyPair.passphrase)")
            lines.append("Companion code: \(keyPair.companionCode)")
        }

        ui.printPanel(title: "Encoded", body: lines.joined(separator: "\n"))
        ui.pause()
    }

    /// Returns `false` (and prints a warning) if `input` exceeds the size limit
    /// and the user declines to proceed.  Returns `true` when the input is within
    /// limits or the user explicitly confirms they want to continue.
    private func confirmLargeInput(_ input: String) -> Bool {
        let bytes = input.utf8.count
        guard bytes > Self.messageSizeLimitBytes else { return true }
        let kb = bytes / 1024
        let answer = ui.prompt(
            "Warning: input is \(kb) KB (limit \(Self.messageSizeLimitBytes / 1024) KB). Proceed? (y/N):"
        )
        return answer.lowercased().hasPrefix("y")
    }

    private func historyEntry(for result: EncodedMessage, operation: OperationKind) -> HistoryEntry {
        HistoryEntry(
            id: UUID(),
            timestamp: Date(),
            operation: operation,
            inputPreview: truncate(result.originalInput),
            outputPreview: truncate(result.securePayload ?? result.transformedText),
            techniques: result.techniques,
            usedSecureEnvelope: result.usedSecureEnvelope
        )
    }

    private func truncate(_ value: String) -> String {
        let cleaned = value.replacingOccurrences(of: "\n", with: " ")
        return cleaned.count > 80 ? String(cleaned.prefix(77)) + "..." : cleaned
    }
}
