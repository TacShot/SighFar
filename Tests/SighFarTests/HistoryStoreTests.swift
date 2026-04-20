import Testing
import Foundation
@testable import SighFarCore

@Suite("HistoryStore")
struct HistoryStoreTests {
    /// Create an isolated store backed by a unique temporary directory so that
    /// tests never share state or touch the real user home directory.
    private func makeStore() -> (HistoryStore, URL) {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("SighFarTests-\(UUID().uuidString)", isDirectory: true)
        return (HistoryStore(baseDirectory: dir), dir)
    }

    private func cleanup(_ dir: URL) {
        try? FileManager.default.removeItem(at: dir)
    }

    private func sampleEntry(op: OperationKind = .encode) -> HistoryEntry {
        HistoryEntry(
            id: UUID(),
            timestamp: Date(),
            operation: op,
            inputPreview: "in",
            outputPreview: "out",
            techniques: [.caesar(shift: 3)],
            usedSecureEnvelope: false
        )
    }

    // MARK: – Empty store

    @Test func emptyStoreReturnsEmptyArray() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        let entries = try store.load()
        #expect(entries.isEmpty)
    }

    // MARK: – Append and load

    @Test func appendThenLoadReturnsSingleEntry() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        let entry = sampleEntry()
        try store.append(entry)
        let loaded = try store.load()
        #expect(loaded.count == 1)
        #expect(loaded[0].id == entry.id)
    }

    @Test func newestEntryFirstOrdering() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        let first = sampleEntry()
        let second = sampleEntry()
        try store.append(first)
        try store.append(second)
        let loaded = try store.load()
        // Most recent append should be at index 0.
        #expect(loaded[0].id == second.id)
        #expect(loaded[1].id == first.id)
    }

    // MARK: – Entry cap

    @Test func entryCapDropsOldest() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        // Append one more than the max.
        let overCount = HistoryStore.maxEntries + 5
        for _ in 0..<overCount {
            try store.append(sampleEntry())
        }
        let loaded = try store.load()
        #expect(loaded.count == HistoryStore.maxEntries)
    }

    // MARK: – loadRecent

    @Test func loadRecentReturnsAtMostLimit() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        for _ in 0..<20 {
            try store.append(sampleEntry())
        }
        let recent = try store.loadRecent(limit: 5)
        #expect(recent.count == 5)
    }

    @Test func loadRecentOnEmptyStoreReturnsEmpty() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        let recent = try store.loadRecent(limit: 10)
        #expect(recent.isEmpty)
    }

    // MARK: – Key persistence

    @Test func keyFileAutoCreatedAndReused() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        let entry = sampleEntry()
        try store.append(entry)
        // A second store pointing at the same directory must decrypt successfully.
        let store2 = HistoryStore(baseDirectory: dir)
        let loaded = try store2.load()
        #expect(loaded.count == 1)
        #expect(loaded[0].id == entry.id)
    }

    // MARK: – Corrupt file

    @Test func corruptHistoryFileThrows() throws {
        let (store, dir) = makeStore()
        defer { cleanup(dir) }
        // Write garbage into the history file.
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let histFile = dir.appendingPathComponent("history.enc")
        try Data("not valid encrypted data".utf8).write(to: histFile)
        #expect(throws: (any Error).self) {
            try store.load()
        }
    }
}
