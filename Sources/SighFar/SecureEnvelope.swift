import CryptoKit
import Foundation

struct SecureEnvelope {
    func seal(_ message: String, keyPair: SecureKeyPair) throws -> String {
        let key = deriveKey(from: keyPair)
        let data = Data(message.utf8)
        let sealed = try AES.GCM.seal(data, using: key)
        guard let combined = sealed.combined else {
            throw CipherError.malformedPayload
        }
        return combined.base64EncodedString()
    }

    func open(_ payload: String, keyPair: SecureKeyPair) throws -> String {
        guard let data = Data(base64Encoded: payload) else {
            throw CipherError.malformedPayload
        }

        let box = try AES.GCM.SealedBox(combined: data)
        let plaintext = try AES.GCM.open(box, using: deriveKey(from: keyPair))
        guard let string = String(data: plaintext, encoding: .utf8) else {
            throw CipherError.malformedPayload
        }
        return string
    }

    func makeKeyPair(passphrase: String) -> SecureKeyPair {
        SecureKeyPair(
            passphrase: passphrase,
            companionCode: Self.randomCode(length: 18)
        )
    }

    private func deriveKey(from keyPair: SecureKeyPair) -> SymmetricKey {
        let seed = Data("\(keyPair.passphrase)|\(keyPair.companionCode)".utf8)
        let digest = SHA256.hash(data: seed)
        return SymmetricKey(data: Data(digest))
    }

    private static func randomCode(length: Int) -> String {
        let alphabet = Array("ABCDEFGHJKLMNPQRSTUVWXYZ23456789")
        var generator = SystemRandomNumberGenerator()
        return String((0..<length).map { _ in
            alphabet.randomElement(using: &generator)!
        })
    }
}
