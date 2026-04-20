import Testing
import Foundation
@testable import SighFarCore

@Suite("CipherPipeline")
struct CipherPipelineTests {
    let pipeline = CipherPipeline()

    // MARK: – Individual roundtrips

    @Test func morseRoundtrip() throws {
        let original = "hello world"
        let encoded = try pipeline.encode(original, using: [.morse])
        let decoded = try pipeline.decode(encoded, using: [.morse])
        #expect(decoded == original)
    }

    @Test func caesarRoundtrip() throws {
        let original = "Hello World"
        let encoded = try pipeline.encode(original, using: [.caesar(shift: 13)])
        let decoded = try pipeline.decode(encoded, using: [.caesar(shift: 13)])
        #expect(decoded == original)
    }

    @Test func vigenereRoundtrip() throws {
        let original = "Hello World"
        let encoded = try pipeline.encode(original, using: [.vigenere(keyword: "key")])
        let decoded = try pipeline.decode(encoded, using: [.vigenere(keyword: "key")])
        #expect(decoded == original)
    }

    @Test func railFenceRoundtrip() throws {
        let original = "Hello World"
        let encoded = try pipeline.encode(original, using: [.railFence(rails: 3)])
        let decoded = try pipeline.decode(encoded, using: [.railFence(rails: 3)])
        #expect(decoded == original)
    }

    @Test func reverseRoundtrip() throws {
        let original = "Hello World"
        let encoded = try pipeline.encode(original, using: [.reverse])
        let decoded = try pipeline.decode(encoded, using: [.reverse])
        #expect(decoded == original)
    }

    // MARK: – Chained stack

    @Test func chainedRoundtrip() throws {
        let original = "hello"
        let techniques: [TechniqueDescriptor] = [.caesar(shift: 4), .reverse]
        let encoded = try pipeline.encode(original, using: techniques)
        let decoded = try pipeline.decode(encoded, using: techniques)
        #expect(decoded == original)
    }

    // MARK: – Edge cases

    @Test func emptyInputCaesar() throws {
        let encoded = try pipeline.encode("", using: [.caesar(shift: 3)])
        #expect(encoded == "")
    }

    @Test func singleCharCaesar() throws {
        let encoded = try pipeline.encode("a", using: [.caesar(shift: 1)])
        #expect(encoded == "b")
    }

    @Test func caesarWrapsZShift1() throws {
        #expect(try pipeline.encode("z", using: [.caesar(shift: 1)]) == "a")
        #expect(try pipeline.encode("Z", using: [.caesar(shift: 1)]) == "A")
    }

    @Test func nonAsciiPassthroughCaesar() throws {
        // Non-ASCII characters should survive unchanged.
        let original = "héllo"
        let encoded = try pipeline.encode(original, using: [.caesar(shift: 3)])
        #expect(encoded.contains("é"))
    }

    @Test func numericInputMorse() throws {
        // Morse supports digits
        let encoded = try pipeline.encode("42", using: [.morse])
        let decoded = try pipeline.decode(encoded, using: [.morse])
        #expect(decoded == "42")
    }

    // MARK: – Error paths

    @Test func unsupportedMorseCharacterThrows() {
        #expect(throws: (any Error).self) {
            try pipeline.encode("hello!", using: [.morse])
        }
    }

    @Test func invalidVigenereKeywordThrows() {
        // A keyword of only digits has no letters → invalid
        #expect(throws: (any Error).self) {
            try pipeline.encode("hello", using: [.vigenere(keyword: "123")])
        }
    }

    @Test func railFenceOneRailThrows() {
        #expect(throws: (any Error).self) {
            try pipeline.encode("test", using: [.railFence(rails: 1)])
        }
    }

    // MARK: – Large input (pipeline itself should handle it; the App layer warns)

    @Test func largeInputCaesarRoundtrip() throws {
        // The pipeline must complete without crashing on large inputs.
        let large = String(repeating: "a", count: 10_000)
        let encoded = try pipeline.encode(large, using: [.caesar(shift: 1)])
        let decoded = try pipeline.decode(encoded, using: [.caesar(shift: 1)])
        #expect(decoded == large)
    }
}
