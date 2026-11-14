/// One step of a path into a value: a map key or an array index.
public enum PathSegment: Sendable, Hashable, CustomStringConvertible {
    case key(String)
    case index(Int)

    public var description: String {
        switch self {
        case .key(let k): return k
        case .index(let i): return String(i)
        }
    }
}

/// A problem found by an operation.
public struct Diagnostic: Sendable, Hashable, CustomStringConvertible {
    /// A lower-case word with hyphens, such as `duplicate-key`.
    public let code: String
    /// Text for a person; not part of the contract.
    public let message: String
    /// The part of the value concerned; empty for the whole value.
    public let path: [PathSegment]

    public init(code: String, message: String, path: [PathSegment] = []) {
        self.code = code
        self.message = message
        self.path = path
    }

    /// The path as a JSON Pointer (RFC 6901): `/items/0/name`, empty for the whole value.
    public var pointer: String { pathToPointer(path) }

    public var description: String {
        path.isEmpty ? "\(code): \(message)" : "\(code): \(message) (at \(pointer))"
    }
}

/// A path as a JSON Pointer (RFC 6901).
public func pathToPointer(_ path: [PathSegment]) -> String {
    var out = ""
    for segment in path {
        out += "/"
        for c in segment.description {
            switch c {
            case "~": out += "~0"
            case "/": out += "~1"
            default: out.append(c)
            }
        }
    }
    return out
}

/// Every problem found by a failed operation. It is the error those operations throw.
public struct Diagnostics: Error, Sendable, Hashable, CustomStringConvertible {
    public let items: [Diagnostic]

    public init(_ items: [Diagnostic]) { self.items = items }

    init(code: String, message: String, path: [PathSegment] = []) {
        self.items = [Diagnostic(code: code, message: message, path: path)]
    }

    public var description: String { items.map(\.description).joined(separator: "; ") }

    /// The same problems with a segment in front of every path.
    func under(_ segment: PathSegment) -> Diagnostics {
        Diagnostics(items.map { Diagnostic(code: $0.code, message: $0.message, path: [segment] + $0.path) })
    }

    /// The same problems at `path`.
    func at(_ path: [PathSegment]) -> Diagnostics {
        Diagnostics(items.map { Diagnostic(code: $0.code, message: $0.message, path: path) })
    }
}
