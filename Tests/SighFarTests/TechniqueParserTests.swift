import Testing
import Foundation
@testable import SighFarCore

@Suite("TechniqueParser")
struct TechniqueParserTests {
    let parser = TechniqueParser()

    // MARK: – Valid inputs

    @Test func parseMorse() throws {
        let result = try parser.parse(from: "morse")
        #expect(result == [.morse])
    }

    @Test func parseReverse() throws {
        let result = try parser.parse(from: "reverse")
        #expect(result == [.reverse])
    }

    @Test func parseCaesar() throws {
        let result = try parser.parse(from: "caesar:7")
        #expect(result == [.caesar(shift: 7)])
    }

    @Test func parseVigenere() throws {
        let result = try parser.parse(from: "vigenere:secret")
        #expect(result == [.vigenere(keyword: "secret")])
    }

    @Test func parseRailFence() throws {
        let result = try parser.parse(from: "railfence:4")
        #expect(result == [.railFence(rails: 4)])
    }

    @Test func parseChain() throws {
        let result = try parser.parse(from: "morse,caesar:4,reverse")
        #expect(result == [.morse, .caesar(shift: 4), .reverse])
    }

    @Test func parseChainWithSpaces() throws {
        // Surrounding whitespace should be trimmed.
        let result = try parser.parse(from: " caesar:3 , reverse ")
        #expect(result == [.caesar(shift: 3), .reverse])
    }

    @Test func vigenereColonInKeyword() throws {
        // The keyword itself may contain colons (maxSplits: 1).
        let result = try parser.parse(from: "vigenere:key:word")
        #expect(result == [.vigenere(keyword: "key:word")])
    }

    // MARK: – Error paths

    @Test func emptyInputThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "")
        }
    }

    @Test func whitespaceOnlyThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "   ")
        }
    }

    @Test func unknownTechniqueThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "base64")
        }
    }

    @Test func malformedCaesarThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "caesar:abc")
        }
    }

    @Test func missingCaesarShiftThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "caesar:")
        }
    }

    @Test func emptyVigenereKeywordThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "vigenere:")
        }
    }

    @Test func malformedRailFenceThrows() {
        #expect(throws: (any Error).self) {
            try parser.parse(from: "railfence:two")
        }
    }
}
