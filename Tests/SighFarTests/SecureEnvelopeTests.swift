import Testing
import Foundation
@testable import SighFarCore

@Suite("SecureEnvelope")
struct SecureEnvelopeTests {
    let envelope = SecureEnvelope()

    // MARK: – Happy path

    @Test func sealOpenRoundtrip() throws {
        let message = "top secret message"
        let keyPair = envelope.makeKeyPair(passphrase: "hunter2")
        let sealed = try envelope.seal(message, keyPair: keyPair)
        let opened = try envelope.open(sealed, keyPair: keyPair)
        #expect(opened == message)
    }

    @Test func sealOpenRoundtripUnicode() throws {
        let message = "こんにちは 🔐"
        let keyPair = envelope.makeKeyPair(passphrase: "passphrase")
        let sealed = try envelope.seal(message, keyPair: keyPair)
        let opened = try envelope.open(sealed, keyPair: keyPair)
        #expect(opened == message)
    }

    // MARK: – Wrong credentials

    @Test func wrongPassphraseThrows() throws {
        let keyPair = envelope.makeKeyPair(passphrase: "correct")
        let sealed = try envelope.seal("secret", keyPair: keyPair)
        let wrongPair = SecureKeyPair(passphrase: "wrong", companionCode: keyPair.companionCode)
        #expect(throws: (any Error).self) {
            try envelope.open(sealed, keyPair: wrongPair)
        }
    }

    @Test func wrongCompanionCodeThrows() throws {
        let keyPair = envelope.makeKeyPair(passphrase: "correct")
        let sealed = try envelope.seal("secret", keyPair: keyPair)
        let wrongPair = SecureKeyPair(passphrase: keyPair.passphrase, companionCode: "AAAAAAAAAAAAAAAAAAA")
        #expect(throws: (any Error).self) {
            try envelope.open(sealed, keyPair: wrongPair)
        }
    }

    @Test func corruptedBase64Throws() {
        let keyPair = envelope.makeKeyPair(passphrase: "pw")
        #expect(throws: (any Error).self) {
            try envelope.open("not-valid-base64!!!", keyPair: keyPair)
        }
    }

    @Test func truncatedPayloadThrows() throws {
        let keyPair = envelope.makeKeyPair(passphrase: "pw")
        let sealed = try envelope.seal("hello", keyPair: keyPair)
        // Truncate to force an invalid sealed box
        let truncated = String(sealed.prefix(8))
        #expect(throws: (any Error).self) {
            try envelope.open(truncated, keyPair: keyPair)
        }
    }

    // MARK: – makeKeyPair

    @Test func companionCodeLength() {
        let pair = envelope.makeKeyPair(passphrase: "any")
        #expect(pair.companionCode.count == 18)
    }

    @Test func companionCodeAlphabet() {
        // Must only contain characters from the defined alphabet.
        let alphabet = Set("ABCDEFGHJKLMNPQRSTUVWXYZ23456789")
        let pair = envelope.makeKeyPair(passphrase: "any")
        for ch in pair.companionCode {
            #expect(alphabet.contains(ch))
        }
    }

    @Test func differentCallsProduceDifferentCodes() {
        // Two calls should (overwhelmingly) produce different codes.
        let codes = (0..<10).map { _ in envelope.makeKeyPair(passphrase: "x").companionCode }
        let unique = Set(codes)
        #expect(unique.count > 1)
    }

    // MARK: – Key derivation determinism

    @Test func keyDerivationDeterministic() throws {
        let pair = SecureKeyPair(passphrase: "pw", companionCode: "ABC123DEF456GHJ789")
        let msg = "test"
        let sealed1 = try envelope.seal(msg, keyPair: pair)
        let sealed2 = try envelope.seal(msg, keyPair: pair)
        // Different seals (random nonce) but both open with the same key.
        #expect(try envelope.open(sealed1, keyPair: pair) == msg)
        #expect(try envelope.open(sealed2, keyPair: pair) == msg)
    }
}
