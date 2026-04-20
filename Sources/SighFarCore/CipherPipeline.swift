import Foundation

enum CipherError: Error, LocalizedError {
    case invalidInput(String)
    case secureEnvelopeUnavailable
    case malformedPayload

    var errorDescription: String? {
        switch self {
        case .invalidInput(let message):
            return message
        case .secureEnvelopeUnavailable:
            return "Secure envelope support is unavailable on this build."
        case .malformedPayload:
            return "The payload could not be parsed."
        }
    }
}

struct CipherPipeline {
    func encode(_ input: String, using techniques: [TechniqueDescriptor]) throws -> String {
        try techniques.reduce(input) { partial, technique in
            try transform(partial, technique: technique, direction: .encode)
        }
    }

    func decode(_ input: String, using techniques: [TechniqueDescriptor]) throws -> String {
        try techniques.reversed().reduce(input) { partial, technique in
            try transform(partial, technique: technique, direction: .decode)
        }
    }

    private func transform(
        _ input: String,
        technique: TechniqueDescriptor,
        direction: OperationKind
    ) throws -> String {
        switch technique {
        case .morse:
            return try MorseCodec().process(input, direction: direction)
        case .caesar(let shift):
            return CaesarCipher(shift: shift).process(input, direction: direction)
        case .vigenere(let keyword):
            return try VigenereCipher(keyword: keyword).process(input, direction: direction)
        case .railFence(let rails):
            return try RailFenceCipher(rails: rails).process(input, direction: direction)
        case .reverse:
            return String(input.reversed())
        }
    }
}

private struct MorseCodec {
    private let encodeMap: [Character: String] = [
        "a": ".-", "b": "-...", "c": "-.-.", "d": "-..", "e": ".", "f": "..-.",
        "g": "--.", "h": "....", "i": "..", "j": ".---", "k": "-.-", "l": ".-..",
        "m": "--", "n": "-.", "o": "---", "p": ".--.", "q": "--.-", "r": ".-.",
        "s": "...", "t": "-", "u": "..-", "v": "...-", "w": ".--", "x": "-..-",
        "y": "-.--", "z": "--..",
        "0": "-----", "1": ".----", "2": "..---", "3": "...--", "4": "....-",
        "5": ".....", "6": "-....", "7": "--...", "8": "---..", "9": "----.",
        " ": "/"
    ]

    func process(_ input: String, direction: OperationKind) throws -> String {
        switch direction {
        case .encode:
            return try input.lowercased().map { character in
                guard let symbol = encodeMap[character] else {
                    throw CipherError.invalidInput("Unsupported Morse character: \(character)")
                }
                return symbol
            }.joined(separator: " ")
        case .decode:
            let decodeMap = Dictionary(uniqueKeysWithValues: encodeMap.map { ($1, $0) })
            let parts = input.split(separator: " ").map(String.init)
            return try parts.map { part in
                guard let character = decodeMap[part] else {
                    throw CipherError.invalidInput("Unsupported Morse token: \(part)")
                }
                return String(character)
            }.joined()
        }
    }
}

private struct CaesarCipher {
    let shift: Int

    func process(_ input: String, direction: OperationKind) -> String {
        let signedShift = direction == .encode ? shift : -shift
        return String(input.map { rotate($0, by: signedShift) })
    }

    private func rotate(_ character: Character, by amount: Int) -> Character {
        guard let scalar = character.unicodeScalars.first, scalar.isASCII else {
            return character
        }

        let value = Int(scalar.value)
        switch value {
        case 65...90:
            return Character(UnicodeScalar(((value - 65 + amount) % 26 + 26) % 26 + 65)!)
        case 97...122:
            return Character(UnicodeScalar(((value - 97 + amount) % 26 + 26) % 26 + 97)!)
        default:
            return character
        }
    }
}

private struct VigenereCipher {
    let keyword: [Int]

    init(keyword: String) throws {
        let normalized = keyword.lowercased().filter(\.isLetter)
        guard !normalized.isEmpty else {
            throw CipherError.invalidInput("Vigenere keyword must include letters.")
        }
        self.keyword = normalized.compactMap { $0.asciiLowercaseOffset }
    }

    func process(_ input: String, direction: OperationKind) -> String {
        var keyIndex = 0

        return String(input.map { character in
            guard let offset = character.asciiLowercaseOffset else {
                return character
            }

            let shift = keyword[keyIndex % keyword.count]
            keyIndex += 1
            let appliedShift = direction == .encode ? shift : -shift
            return rotate(character, by: appliedShift, originalOffset: offset)
        })
    }

    private func rotate(_ character: Character, by amount: Int, originalOffset: Int) -> Character {
        let isUppercase = character.unicodeScalars.first?.value ?? 97 < 97
        let base = isUppercase ? 65 : 97
        let rotated = ((originalOffset + amount) % 26 + 26) % 26 + base
        return Character(UnicodeScalar(rotated)!)
    }
}

private struct RailFenceCipher {
    let rails: Int

    init(rails: Int) throws {
        guard rails >= 2 else {
            throw CipherError.invalidInput("RailFence requires at least 2 rails.")
        }
        self.rails = rails
    }

    func process(_ input: String, direction: OperationKind) throws -> String {
        switch direction {
        case .encode:
            return encode(input)
        case .decode:
            return decode(input)
        }
    }

    private func encode(_ input: String) -> String {
        guard input.count > 1 else { return input }

        var fence = Array(repeating: "", count: rails)
        var rail = 0
        var direction = 1

        for character in input {
            fence[rail].append(character)
            if rail == 0 {
                direction = 1
            } else if rail == rails - 1 {
                direction = -1
            }
            rail += direction
        }

        return fence.joined()
    }

    private func decode(_ input: String) -> String {
        guard input.count > 1 else { return input }

        let pattern = railPattern(length: input.count)
        var railCounts = Array(repeating: 0, count: rails)
        for rail in pattern {
            railCounts[rail] += 1
        }

        var railSlices: [[Character]] = []
        var cursor = input.startIndex
        for count in railCounts {
            let end = input.index(cursor, offsetBy: count)
            railSlices.append(Array(input[cursor..<end]))
            cursor = end
        }

        var railOffsets = Array(repeating: 0, count: rails)
        var output = ""
        output.reserveCapacity(input.count)

        for rail in pattern {
            output.append(railSlices[rail][railOffsets[rail]])
            railOffsets[rail] += 1
        }

        return output
    }

    private func railPattern(length: Int) -> [Int] {
        var pattern: [Int] = []
        pattern.reserveCapacity(length)

        var rail = 0
        var direction = 1
        for _ in 0..<length {
            pattern.append(rail)
            if rail == 0 {
                direction = 1
            } else if rail == rails - 1 {
                direction = -1
            }
            rail += direction
        }

        return pattern
    }
}

private extension Character {
    var asciiLowercaseOffset: Int? {
        guard let scalar = unicodeScalars.first, scalar.isASCII else { return nil }
        let value = Int(scalar.value)
        switch value {
        case 65...90:
            return value - 65
        case 97...122:
            return value - 97
        default:
            return nil
        }
    }
}
