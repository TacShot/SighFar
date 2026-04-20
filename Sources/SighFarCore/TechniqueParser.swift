import Foundation

/// Parses a comma-separated technique chain string into ``TechniqueDescriptor`` values.
///
/// Extracted from ``SighFarApp`` so it can be tested in isolation without any
/// I/O dependency.
///
/// ## Format
/// ```
/// morse
/// caesar:3
/// vigenere:keyword
/// railfence:4
/// reverse
/// ```
/// Techniques may be combined with commas: `morse,caesar:4,reverse`
struct TechniqueParser {
    func parse(from input: String) throws -> [TechniqueDescriptor] {
        let components = input
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }

        guard !components.isEmpty else {
            throw CipherError.invalidInput("You need at least one technique.")
        }

        return try components.map { component in
            if component == "morse" {
                return .morse
            }
            if component == "reverse" {
                return .reverse
            }
            if component.hasPrefix("caesar:") {
                guard let shift = Int(component.split(separator: ":").last ?? "") else {
                    throw CipherError.invalidInput("Caesar format is caesar:3")
                }
                return .caesar(shift: shift)
            }
            if component.hasPrefix("vigenere:") {
                let parts = component.split(separator: ":", maxSplits: 1).map(String.init)
                guard parts.count == 2, !parts[1].isEmpty else {
                    throw CipherError.invalidInput("Vigenere format is vigenere:keyword")
                }
                return .vigenere(keyword: parts[1])
            }
            if component.hasPrefix("railfence:") {
                guard let rails = Int(component.split(separator: ":").last ?? "") else {
                    throw CipherError.invalidInput("RailFence format is railfence:3")
                }
                return .railFence(rails: rails)
            }

            throw CipherError.invalidInput("Unknown technique: \(component)")
        }
    }
}
