import Foundation

enum OperationKind: String, Codable {
    case encode
    case decode
}

struct SecureKeyPair: Codable {
    let passphrase: String
    let companionCode: String
}

struct EncodedMessage: Codable {
    let originalInput: String
    let transformedText: String
    let securePayload: String?
    let techniques: [TechniqueDescriptor]
    let usedSecureEnvelope: Bool
    let keyPair: SecureKeyPair?
}

struct HistoryEntry: Codable {
    let id: UUID
    let timestamp: Date
    let operation: OperationKind
    let inputPreview: String
    let outputPreview: String
    let techniques: [TechniqueDescriptor]
    let usedSecureEnvelope: Bool
}

enum TechniqueDescriptor: Codable, Equatable {
    case morse
    case caesar(shift: Int)
    case vigenere(keyword: String)
    case railFence(rails: Int)
    case reverse

    var title: String {
        switch self {
        case .morse:
            return "Morse"
        case .caesar(let shift):
            return "Caesar(\(shift))"
        case .vigenere(let keyword):
            return "Vigenere(\(keyword))"
        case .railFence(let rails):
            return "RailFence(\(rails))"
        case .reverse:
            return "Reverse"
        }
    }
}
