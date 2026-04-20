import Crypto
import Foundation

struct HistoryStore {
    /// Maximum number of history entries kept on disk.  Oldest entries are
    /// dropped when this limit is exceeded, keeping storage and decrypt/re-encrypt
    /// time bounded.
    static let maxEntries = 500

    private let fileManager = FileManager.default
    private let supportDirectory: URL
    private let keyFile: URL
    private let historyFile: URL
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    init(baseDirectory: URL? = nil) {
        let root = baseDirectory ?? PlatformPaths().sighfarDirectory
        self.supportDirectory = root
        self.keyFile = root.appendingPathComponent("history.key")
        self.historyFile = root.appendingPathComponent("history.enc")
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        decoder.dateDecodingStrategy = .iso8601
    }

    func append(_ entry: HistoryEntry) throws {
        var items = try load()
        items.insert(entry, at: 0)
        // Enforce the entry cap — drop the oldest entries beyond the limit.
        if items.count > Self.maxEntries {
            items = Array(items.prefix(Self.maxEntries))
        }
        try save(items)
    }

    func load() throws -> [HistoryEntry] {
        try createDirectoryIfNeeded()
        guard fileManager.fileExists(atPath: historyFile.path) else {
            return []
        }

        let payload = try Data(contentsOf: historyFile)
        guard !payload.isEmpty else {
            return []
        }

        let key = try historyKey()
        let sealedBox = try AES.GCM.SealedBox(combined: payload)
        let plaintext = try AES.GCM.open(sealedBox, using: key)
        return try decoder.decode([HistoryEntry].self, from: plaintext)
    }

    /// Returns up to `limit` of the most recent entries without decoding
    /// the full collection beyond what is needed.
    func loadRecent(limit: Int) throws -> [HistoryEntry] {
        let all = try load()
        return Array(all.prefix(limit))
    }

    func diagnostics() -> String {
        """
        storage: \(supportDirectory.path)
        key: \(keyFile.lastPathComponent)
        history: \(historyFile.lastPathComponent)
        max entries: \(Self.maxEntries)
        """
    }

    private func save(_ entries: [HistoryEntry]) throws {
        try createDirectoryIfNeeded()
        let key = try historyKey()
        let data = try encoder.encode(entries)
        let sealed = try AES.GCM.seal(data, using: key)
        guard let combined = sealed.combined else {
            throw CipherError.malformedPayload
        }
        try combined.write(to: historyFile, options: .atomic)
    }

    private func historyKey() throws -> SymmetricKey {
        try createDirectoryIfNeeded()

        if fileManager.fileExists(atPath: keyFile.path) {
            let existing = try Data(contentsOf: keyFile)
            return SymmetricKey(data: existing)
        }

        let key = SymmetricKey(size: .bits256)
        let data = key.withUnsafeBytes { Data($0) }
        try data.write(to: keyFile, options: .atomic)
        return key
    }

    private func createDirectoryIfNeeded() throws {
        try fileManager.createDirectory(
            at: supportDirectory,
            withIntermediateDirectories: true
        )
    }
}
