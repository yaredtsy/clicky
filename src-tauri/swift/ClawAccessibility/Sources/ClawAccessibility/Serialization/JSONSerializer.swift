import Foundation

/// Serialize an AXTreeResponse to a JSON string.
///
/// Uses Foundation's JSONEncoder with:
/// - No pretty printing (smaller payload for IPC)
/// - Sorted keys (deterministic output, useful for diffing/debugging)
///
/// ## Why Foundation Codable?
///
/// 1. Zero manual string building = zero escaping bugs
/// 2. Type-safe: compiler ensures all fields are encoded
/// 3. Adding a new field to AXNodeModel automatically includes it in JSON
/// 4. Matches Rust's serde_json deserialization exactly
enum JSONSerializer {
    
    /// Serialize the full tree response to JSON.
    ///
    /// Returns the JSON string, or an error message prefixed with "error:".
    static func serialize(_ response: AXTreeResponse) -> String {
        let encoder = JSONEncoder()
        // Use sorted keys for deterministic output
        encoder.outputFormatting = [.sortedKeys]
        
        do {
            let data = try encoder.encode(response)
            return String(data: data, encoding: .utf8) ?? "error:UTF-8 encoding failed"
        } catch {
            return "error:JSON encoding failed: \(error.localizedDescription)"
        }
    }
    
    /// Serialize just a single node (useful for partial updates in the future).
    static func serializeNode(_ node: AXNodeModel) -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        
        do {
            let data = try encoder.encode(node)
            return String(data: data, encoding: .utf8) ?? "error:UTF-8 encoding failed"
        } catch {
            return "error:JSON encoding failed: \(error.localizedDescription)"
        }
    }
}