import Foundation

/// Serialize an AXTreeResponse to XML string.
///
/// This is kept for backward compatibility and file export.
/// The primary IPC format is JSON (see JSONSerializer).
enum XMLSerializer {
    
    static func serialize(_ response: AXTreeResponse) -> String {
        var xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        xml += "<accessibility"
        xml += " pid=\"\(response.app.pid)\""
        if let bid = response.app.bundleIdentifier {
            xml += attr("bundleIdentifier", bid)
        }
        if let name = response.app.name {
            xml += attr("name", name)
        }
        xml += " nodes=\"\(response.nodeCount)\""
        xml += ">\n"
        serializeNode(response.root, into: &xml)
        xml += "\n</accessibility>\n"
        return xml
    }
    
    // MARK: - Private
    
    private static func serializeNode(_ node: AXNodeModel, into xml: inout String) {
        xml += "<node"
        xml += attr("role", node.role)
        xml += optAttr("subrole", node.subrole)
        xml += optAttr("title", node.title)
        xml += optAttr("label", node.label)
        xml += optAttr("description", node.description)
        xml += optAttr("help", node.help)
        xml += optAttr("value", node.value)
        
        if let f = node.frame {
            let frameStr = String(
                format: "{\"x\":%.2f,\"y\":%.2f,\"width\":%.2f,\"height\":%.2f}",
                f.x, f.y, f.width, f.height
            )
            xml += attr("frame", frameStr)
        }
        
        if let e = node.enabled { xml += " enabled=\"\(e)\"" }
        if let f = node.focused { xml += " focused=\"\(f)\"" }
        if let s = node.selected { xml += " selected=\"\(s)\"" }
        
        if !node.actions.isEmpty {
            xml += attr("actions", node.actions.joined(separator: ","))
        }
        
        if node.children.isEmpty {
            xml += "/>"
        } else {
            xml += ">"
            for child in node.children {
                serializeNode(child, into: &xml)
            }
            xml += "</node>"
        }
    }
    
    private static func xmlEscape(_ s: String) -> String {
        var out = ""
        out.reserveCapacity(s.count)
        for ch in s.unicodeScalars {
            switch ch {
            case "&": out += "&amp;"
            case "<": out += "&lt;"
            case ">": out += "&gt;"
            case "\"": out += "&quot;"
            case "'": out += "&apos;"
            default:
                if ch.value < 0x20 && ch != "\n" && ch != "\r" && ch != "\t" {
                    out += "&#\(ch.value);"
                } else {
                    out.unicodeScalars.append(ch)
                }
            }
        }
        return out
    }
    
    private static func attr(_ name: String, _ value: String) -> String {
        " \(name)=\"\(xmlEscape(value))\""
    }
    
    private static func optAttr(_ name: String, _ value: String?) -> String {
        guard let v = value, !v.isEmpty else { return "" }
        return attr(name, v)
    }
}