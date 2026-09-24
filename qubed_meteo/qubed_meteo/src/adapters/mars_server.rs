//! Adapter that builds a [`Qube`] by exhaustively traversing a live MARS
//! catalogue server using the marstools tagged-binary TCP protocol.
//!
//! # Wire protocol overview
//!
//! Every value on the wire is preceded by a 1-byte **tag** that identifies
//! its type.  The tags used by the MARS catalogue server are:
//!
//! | Tag | Name                  | Payload                                     |
//! |-----|-----------------------|---------------------------------------------|
//! |   1 | `TAG_START_OBJ`       | (none – object header)                      |
//! |   2 | `TAG_END_OBJ`         | (none – object footer; silently discarded)  |
//! |   5 | `TAG_INT`             | 4 bytes, big-endian u32                     |
//! |  10 | `TAG_UNSIGNED_LONG`   | 4 bytes, big-endian u32                     |
//! |  12 | `TAG_UNSIGNED_LONG_LONG` | 8 bytes, big-endian (hi-word, lo-word)   |
//! |  15 | `TAG_STRING`          | 4-byte length + `length` bytes UTF-8        |
//!
//! Importantly, `read_tag` silently skips any `TAG_END_OBJ` bytes before
//! reading the expected tag, exactly mirroring `Stream.read_tag()` in
//! `marstools/streaming.py`.
//!
//! # Node types
//!
//! Each server connection resolves a `ref` (an opaque string key) to one of
//! the following node handler classes, decoded into [`FetchedData`]:
//!
//! | Handler class        | Meaning                                              |
//! |----------------------|------------------------------------------------------|
//! | `PSimpleNode`        | A dimension with named values → child refs          |
//! | `PSimpleNodeDefault` | Same as `PSimpleNode` but with a server-side default |
//! | `PBranchNode`        | Conditional routing; both branches are followed     |
//! | `PResearchNode`      | Interactive experiment-version lookup               |
//! | `PBalanceNode`       | Treated identically to `PSimpleNode`                |
//! | `PLeafNode`          | Forwarding pointer to a shape node                  |
//! | `PMonoAxisShape`     | Leaf: `(axis_name, [values])` pairs                 |
//! | `PBufrShape`         | Same wire format as `PMonoAxisShape`                |
//! | `PShape`             | Same wire format as `PMonoAxisShape`                |

use std::io::{self, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use qubed::{Coordinates, NodeIdx, Qube};

// ── Wire-protocol constants ───────────────────────────────────────────────────

const TAG_START_OBJ: u8 = 1;
const TAG_END_OBJ: u8 = 2;
const TAG_INT: u8 = 5;
const TAG_UNSIGNED_LONG: u8 = 10;
const TAG_UNSIGNED_LONG_LONG: u8 = 12;
const TAG_STRING: u8 = 15;

/// The MARS server sends `u32::MAX` as the `n` field of a `PResearchNode` to
/// signal that the accumulated prefix has uniquely resolved one experiment
/// version and the next node ref follows immediately.
const RESEARCH_TERMINAL: u32 = u32::MAX;

// ── MarsStream ────────────────────────────────────────────────────────────────

/// Typed read/write wrapper over the marstools binary protocol.
///
/// The type parameters `R` and `W` allow tests to substitute in-memory
/// buffers without changing any protocol or Qube-building logic.
struct MarsStream<R: Read, W: Write> {
    reader: R,
    writer: W,
}

// Production constructor (TcpStream)
impl MarsStream<BufReader<TcpStream>, TcpStream> {
    fn connect(host: &str, port: u16) -> io::Result<Self> {
        let addr_str = format!("{host}:{port}");
        let sock_addr = addr_str
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no address resolved"))?;

        let stream = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);
        Ok(MarsStream { reader, writer })
    }
}

impl<R: Read, W: Write> MarsStream<R, W> {
    #[cfg(test)]
    fn new(reader: R, writer: W) -> Self {
        MarsStream { reader, writer }
    }

    // ── Low-level I/O ─────────────────────────────────────────────────────────

    fn read_exact_n(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read one byte, silently discarding any `TAG_END_OBJ` bytes until the
    /// expected tag is found.  Mirrors `Stream.read_tag()` in streaming.py.
    fn read_tag(&mut self, expected: u8) -> io::Result<()> {
        loop {
            let mut b = [0u8; 1];
            self.reader.read_exact(&mut b)?;
            match b[0] {
                TAG_END_OBJ => continue,
                t if t == expected => return Ok(()),
                t => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("expected tag {expected} ({:?}), got {t}", tag_name(expected)),
                    ))
                }
            }
        }
    }

    fn write_tag(&mut self, tag: u8) -> io::Result<()> {
        self.writer.write_all(&[tag])
    }

    // ── Typed readers ─────────────────────────────────────────────────────────

    /// `TAG_INT(5)` + 4-byte big-endian u32
    fn read_int(&mut self) -> io::Result<u32> {
        self.read_tag(TAG_INT)?;
        let b = self.read_exact_n(4)?;
        Ok(u32::from_be_bytes(b.try_into().unwrap()))
    }

    /// `TAG_UNSIGNED_LONG(10)` + 4-byte big-endian u32
    fn read_unsigned_long(&mut self) -> io::Result<u32> {
        self.read_tag(TAG_UNSIGNED_LONG)?;
        let b = self.read_exact_n(4)?;
        Ok(u32::from_be_bytes(b.try_into().unwrap()))
    }

    /// `TAG_UNSIGNED_LONG_LONG(12)` + 4-byte hi-word + 4-byte lo-word, big-endian
    fn read_unsigned_long_long(&mut self) -> io::Result<u64> {
        self.read_tag(TAG_UNSIGNED_LONG_LONG)?;
        let hi = u32::from_be_bytes(self.read_exact_n(4)?.try_into().unwrap());
        let lo = u32::from_be_bytes(self.read_exact_n(4)?.try_into().unwrap());
        Ok(((hi as u64) << 32) | lo as u64)
    }

    /// `TAG_STRING(15)` + 4-byte big-endian length + `length` bytes UTF-8
    fn read_string(&mut self) -> io::Result<String> {
        self.read_tag(TAG_STRING)?;
        let lb = self.read_exact_n(4)?;
        let len = u32::from_be_bytes(lb.try_into().unwrap()) as usize;
        let data = self.read_exact_n(len)?;
        String::from_utf8(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    // ── Typed writers ─────────────────────────────────────────────────────────

    /// `TAG_STRING(15)` + 4-byte big-endian length + `length` bytes UTF-8
    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_tag(TAG_STRING)?;
        let n = s.len() as u32;
        self.writer.write_all(&n.to_be_bytes())?;
        self.writer.write_all(s.as_bytes())
    }

    // ── Object framing ────────────────────────────────────────────────────────

    /// Read `TAG_START_OBJ` then the object class-name string.
    ///
    /// Returns `None` on EOF (the server indicates the ref is absent from
    /// the catalogue).
    fn read_object(&mut self) -> io::Result<Option<String>> {
        let mut b = [0u8; 1];
        match self.reader.read_exact(&mut b) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
            Ok(()) => {}
        }
        match b[0] {
            0 => Ok(None), // null / EOF sentinel
            TAG_START_OBJ => Ok(Some(self.read_string()?)),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("read_object: expected TAG_START_OBJ({TAG_START_OBJ}), got {other}"),
            )),
        }
    }

    /// `TAG_START_OBJ` + `write_string(name)` + `write_string(ref_)` + `TAG_END_OBJ`
    fn write_object(&mut self, name: &str, ref_: &str) -> io::Result<()> {
        self.write_tag(TAG_START_OBJ)?;
        self.write_string(name)?;
        self.write_string(ref_)?;
        self.write_tag(TAG_END_OBJ)
    }
}

fn tag_name(tag: u8) -> &'static str {
    match tag {
        0 => "zero",
        TAG_START_OBJ => "start_obj",
        TAG_END_OBJ => "end_obj",
        TAG_INT => "int",
        TAG_UNSIGNED_LONG => "unsigned_long",
        TAG_UNSIGNED_LONG_LONG => "unsigned_long_long",
        TAG_STRING => "string",
        _ => "unknown",
    }
}

// ── Decoded node data ─────────────────────────────────────────────────────────

/// Decoded payload of a single MARS catalogue node.
///
/// Each variant corresponds to one or more handler classes in the Python
/// reference implementation.
#[derive(Debug, PartialEq)]
enum FetchedData {
    /// `PSimpleNode` / `PSimpleNodeDefault` / `PBalanceNode`:
    /// a dimension with a finite enumeration of named values and child refs.
    Simple {
        /// The dimension name at this level (e.g. `"class"`, `"stream"`).
        name: String,
        /// `(value, child_ref)` pairs in wire order.
        children: Vec<(String, String)>,
    },

    /// `PBranchNode`: conditional routing.
    ///
    /// Both branches are followed under the same parent so that `compress()`
    /// can later merge the resulting parallel subtrees.
    Branch {
        /// The branch condition, e.g. `"%param%==251"`.
        expr: String,
        /// Ref for the branch where the condition is satisfied.
        true_ref: String,
        /// Ref for the default (condition not satisfied) branch.
        false_ref: String,
    },

    /// `PResearchNode` with `n > 0`: the server has returned a list of valid
    /// experiment-version strings.  Each must be sent back character-by-character
    /// to resolve its terminal ref (see [`traverse_research_expver`]).
    Research {
        /// The dimension name (typically `"expver"`).
        name: String,
        /// All experiment versions available at this node.
        expvers: Vec<String>,
    },

    /// `PResearchNode` with `n == u32::MAX`: the research traversal is complete
    /// and the following node ref has been sent by the server.
    ResearchTerminal {
        /// Ref of the next node to fetch.
        next_ref: String,
    },

    /// `PLeafNode`: a forwarding pointer to a shape node.
    Redirect {
        shape_ref: String,
    },

    /// `PMonoAxisShape` / `PBufrShape` / `PShape`: the terminal leaf containing
    /// the actual coordinate axes.
    Leaf {
        /// `(axis_name, axis_values)` pairs, in wire order.
        axes: Vec<(String, Vec<String>)>,
    },
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// If `params` contains VOR (`138`) and DIV (`155`) but not U (`131`) and
/// V (`132`), append `"131"` and `"132"`.
///
/// Mirrors `adduv()` in the Python reference implementation.
fn adduv(params: &mut Vec<String>) {
    let has_vo_d = (params.iter().any(|p| p == "138") && params.iter().any(|p| p == "155"))
        || (params.iter().any(|p| p == "138.128")
            && params.iter().any(|p| p == "155.128"));

    let has_u_v = (params.iter().any(|p| p == "131") && params.iter().any(|p| p == "132"))
        || (params.iter().any(|p| p == "131.128")
            && params.iter().any(|p| p == "132.128"));

    if has_vo_d && !has_u_v {
        params.push("131".to_string());
        params.push("132".to_string());
    }
}

/// Convert a slice of string values into a [`Coordinates`] object using the
/// same heuristics as the other `qubed_meteo` adapters:
///
/// * strings with a leading zero digit (e.g. `"0001"`) → `StringCoordinates`
/// * parseable as `i32` → `IntegerCoordinates`
/// * parseable as `f64` → `FloatCoordinates`
/// * everything else → `StringCoordinates`
fn make_coords(vals: &[&str]) -> Option<Coordinates> {
    let mut coords = Coordinates::new();
    for &v in vals {
        let s = v.trim();
        if s.is_empty() {
            continue;
        }
        let leading_zero = s.len() > 1
            && s.starts_with('0')
            && s.chars().nth(1).map_or(false, |c| c.is_ascii_digit());

        if leading_zero {
            coords.append(s.to_string());
        } else if let Ok(i) = s.parse::<i32>() {
            coords.append(i);
        } else if let Ok(f) = s.parse::<f64>() {
            coords.append(f);
        } else {
            coords.append(s.to_string());
        }
    }
    if coords.is_empty() { None } else { Some(coords) }
}

// ── Node decoder ──────────────────────────────────────────────────────────────

/// Decode the node payload from an already-opened `stream`, given the handler
/// class name `handler` that was read from `stream.read_object()`.
///
/// `arg` is forwarded to `PResearchNode` as the experiment-version prefix to
/// send back to the server (empty string for the initial enumeration fetch).
///
/// This is a free function so that tests can drive it with an in-memory
/// `Cursor` rather than a live socket.
fn decode_node_handler<R: Read, W: Write>(
    handler: &str,
    stream: &mut MarsStream<R, W>,
    arg: &str,
) -> Result<Option<FetchedData>, String> {
    let io_err = |e: io::Error| format!("I/O error in handler {handler}: {e}");

    match handler {
        // ── Simple dimension nodes ──────────────────────────────────────────
        "PSimpleNode" | "PBalanceNode" => {
            let name = stream.read_string().map_err(io_err)?;
            let count = stream.read_unsigned_long().map_err(io_err)? as usize;
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                let value = stream.read_string().map_err(io_err)?;
                let child_ref = stream.read_string().map_err(io_err)?;
                children.push((value, child_ref));
            }
            Ok(Some(FetchedData::Simple { name, children }))
        }

        "PSimpleNodeDefault" => {
            let name = stream.read_string().map_err(io_err)?;
            // The server-side default value is internal and not exposed externally.
            let _default_val = stream.read_string().map_err(io_err)?;
            let count = stream.read_unsigned_long().map_err(io_err)? as usize;
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                let value = stream.read_string().map_err(io_err)?;
                let child_ref = stream.read_string().map_err(io_err)?;
                children.push((value, child_ref));
            }
            Ok(Some(FetchedData::Simple { name, children }))
        }

        // ── Branch node ─────────────────────────────────────────────────────
        "PBranchNode" => {
            let expr = stream.read_string().map_err(io_err)?;
            let true_ref = stream.read_string().map_err(io_err)?;
            let false_ref = stream.read_string().map_err(io_err)?;
            Ok(Some(FetchedData::Branch { expr, true_ref, false_ref }))
        }

        // ── Research node (interactive experiment-version lookup) ────────────
        "PResearchNode" => {
            let name = stream.read_string().map_err(io_err)?;
            // Bidirectional exchange: we send the current prefix, the server
            // replies with how many experiment versions match it.
            stream.write_string(arg).map_err(io_err)?;
            let n = stream.read_int().map_err(io_err)?;

            if n == 0 {
                // The prefix is invalid; no experiment versions match.
                Ok(None)
            } else if n == RESEARCH_TERMINAL {
                // The prefix uniquely identifies one experiment version.
                // The server immediately sends the ref for the next node.
                let next_ref = stream.read_string().map_err(io_err)?;
                Ok(Some(FetchedData::ResearchTerminal { next_ref }))
            } else {
                // The server returns a list of all matching experiment versions.
                let mut expvers = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    expvers.push(stream.read_string().map_err(io_err)?);
                }
                Ok(Some(FetchedData::Research { name, expvers }))
            }
        }

        // ── Leaf-forwarding node ────────────────────────────────────────────
        "PLeafNode" => {
            // Skip the 64-bit size field; we only need the shape ref.
            let _size = stream.read_unsigned_long_long().map_err(io_err)?;
            let shape_ref = stream.read_string().map_err(io_err)?;
            Ok(Some(FetchedData::Redirect { shape_ref }))
        }

        // ── Leaf / shape nodes ──────────────────────────────────────────────
        // PBufrShape and PShape share the wire format of PMonoAxisShape.
        "PMonoAxisShape" | "PBufrShape" | "PShape" => {
            let mut axes: Vec<(String, Vec<String>)> = Vec::new();
            loop {
                let val_count = stream.read_unsigned_long().map_err(io_err)? as usize;
                if val_count == 0 {
                    break; // terminator
                }
                let axis_name = stream.read_string().map_err(io_err)?;
                let mut axis_values = Vec::with_capacity(val_count);
                for _ in 0..val_count {
                    axis_values.push(stream.read_string().map_err(io_err)?);
                }
                // Inject derived U/V wind components when VOR/DIV are present.
                if axis_name == "param" {
                    adduv(&mut axis_values);
                }
                axes.push((axis_name, axis_values));
            }
            Ok(Some(FetchedData::Leaf { axes }))
        }

        other => Err(format!("Unknown MARS node handler: '{other}'")),
    }
}

// ── Network layer ─────────────────────────────────────────────────────────────

/// Open a fresh TCP connection to `host:port`, send the `FetchAgent` request
/// for `ref_`, and decode the server response.
///
/// `arg` is passed through to `PResearchNode` (use `""` for all other nodes).
///
/// Returns `Ok(None)` when the server indicates `ref_` is absent from the
/// catalogue.
fn fetch_node(host: &str, port: u16, ref_: &str, arg: &str) -> Result<Option<FetchedData>, String> {
    let mut stream = MarsStream::connect(host, port)
        .map_err(|e| format!("connect to {host}:{port}: {e}"))?;

    stream
        .write_object("FetchAgent", ref_)
        .map_err(|e| format!("write_object: {e}"))?;

    // The server always sends an INT password request; we ignore it.
    stream.read_int().map_err(|e| format!("password-request read: {e}"))?;

    match stream.read_object().map_err(|e| format!("handler-name read: {e}"))? {
        None => Ok(None),
        Some(handler) => decode_node_handler(&handler, &mut stream, arg),
    }
}

// ── Research-node traversal ───────────────────────────────────────────────────

/// Resolve `expver` to the ref of the following node by feeding the characters
/// of `expver` to the server one-at-a-time (accumulating a prefix) until the
/// server replies with `RESEARCH_TERMINAL`.
///
/// Returns `None` if the server declares the prefix invalid at any point.
fn traverse_research_expver(
    host: &str,
    port: u16,
    research_ref: &str,
    expver: &str,
) -> Result<Option<String>, String> {
    let mut prefix = String::with_capacity(expver.len());

    for ch in expver.chars() {
        prefix.push(ch);

        match fetch_node(host, port, research_ref, &prefix)? {
            None => return Ok(None), // prefix declared invalid
            Some(FetchedData::ResearchTerminal { next_ref }) => return Ok(Some(next_ref)),
            Some(FetchedData::Research { .. }) => {
                // Prefix still ambiguous; continue sending more characters.
            }
            Some(other) => {
                return Err(format!(
                    "Unexpected node type during research traversal for expver '{expver}': {other:?}"
                ));
            }
        }
    }

    // Exhausted all characters without reaching terminal: expver is incomplete.
    Ok(None)
}

// ── Qube builder ─────────────────────────────────────────────────────────────

/// Fetch the node at `ref_` (with optional research prefix `arg`), decode it,
/// and recursively insert all reachable paths into `qube` under `parent`.
fn build_subtree(
    host: &str,
    port: u16,
    ref_: &str,
    arg: &str,
    qube: &mut Qube,
    parent: NodeIdx,
) -> Result<(), String> {
    let data = match fetch_node(host, port, ref_, arg)? {
        None => return Ok(()),
        Some(d) => d,
    };

    match data {
        // ── Simple dimension: one child per value ─────────────────────────
        FetchedData::Simple { name, children } => {
            for (value, child_ref) in children {
                let coords = make_coords(&[value.as_str()]);
                let child = qube
                    .get_or_create_child(&name, parent, coords)
                    .map_err(|e| format!("get_or_create_child({name}={value}): {e:?}"))?;
                build_subtree(host, port, &child_ref, "", qube, child)?;
            }
        }

        // ── Branch: follow both branches under the same parent ───────────
        //
        // Both subtrees are inserted at the same level; `compress()` will
        // later merge structurally equivalent sibling nodes.  This correctly
        // enumerates all data regardless of branch direction.
        FetchedData::Branch { expr: _, true_ref, false_ref } => {
            if !true_ref.is_empty() {
                build_subtree(host, port, &true_ref, "", qube, parent)?;
            }
            if !false_ref.is_empty() {
                build_subtree(host, port, &false_ref, "", qube, parent)?;
            }
        }

        // ── Research: enumerate all experiment versions ───────────────────
        FetchedData::Research { name, expvers } => {
            for expver in expvers {
                let coords = make_coords(&[expver.as_str()]);
                let child = qube
                    .get_or_create_child(&name, parent, coords)
                    .map_err(|e| format!("get_or_create_child({name}={expver}): {e:?}"))?;

                if let Some(next_ref) = traverse_research_expver(host, port, ref_, &expver)? {
                    build_subtree(host, port, &next_ref, "", qube, child)?;
                }
            }
        }

        // Reached a terminal during research traversal outside the normal
        // enumerate→traverse flow (e.g. expver has only one character).
        FetchedData::ResearchTerminal { next_ref } => {
            build_subtree(host, port, &next_ref, "", qube, parent)?;
        }

        // ── Leaf forwarding: follow the shape ref ─────────────────────────
        FetchedData::Redirect { shape_ref } => {
            build_subtree(host, port, &shape_ref, "", qube, parent)?;
        }

        // ── Leaf: build a coordinate chain for each axis ──────────────────
        FetchedData::Leaf { axes } => {
            let mut current = parent;
            for (axis_name, axis_values) in axes {
                let vals: Vec<&str> = axis_values.iter().map(String::as_str).collect();
                let coords = make_coords(&vals);
                current = qube
                    .get_or_create_child(&axis_name, current, coords)
                    .map_err(|e| format!("get_or_create_child({axis_name}): {e:?}"))?;
            }
        }
    }

    Ok(())
}

// ── Public trait ─────────────────────────────────────────────────────────────

/// Build a [`Qube`] by exhaustively traversing a live MARS catalogue server.
///
/// The server must speak the marstools tagged-binary protocol over TCP.
/// All dimension names and values are discovered automatically; no query
/// filter is applied.
///
/// # Example
///
/// ```rust,no_run
/// use qubed::Qube;
/// use qubed_meteo::adapters::mars_server::FromMarsServer;
///
/// let qube = Qube::from_mars_server("mars.example.com", 9000)
///     .expect("failed to traverse MARS catalogue");
/// println!("{}", qube.to_ascii());
/// ```
pub trait FromMarsServer {
    /// Connect to the MARS catalogue server at `host:port` and traverse its
    /// tree completely, returning a [`Qube`] that contains all available data
    /// paths.
    ///
    /// # Arguments
    /// * `host` – Hostname or IP address of the MARS catalogue server.
    /// * `port` – TCP port the server is listening on.
    fn from_mars_server(host: &str, port: u16) -> Result<Qube, String>;
}

impl FromMarsServer for Qube {
    fn from_mars_server(host: &str, port: u16) -> Result<Qube, String> {
        let mut qube = Qube::new();
        let root = qube.root();
        // Fetch the root node using an empty ref (the MARS convention for the
        // catalogue root) and recursively build the entire tree.
        build_subtree(host, port, "", "", &mut qube, root)?;
        qube.compress();
        Ok(qube)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ── Wire-encoding helpers ─────────────────────────────────────────────────

    fn enc_tag(tag: u8) -> Vec<u8> {
        vec![tag]
    }

    fn enc_string(s: &str) -> Vec<u8> {
        let mut v = enc_tag(TAG_STRING);
        v.extend_from_slice(&(s.len() as u32).to_be_bytes());
        v.extend_from_slice(s.as_bytes());
        v
    }

    fn enc_u32_tagged(tag: u8, n: u32) -> Vec<u8> {
        let mut v = enc_tag(tag);
        v.extend_from_slice(&n.to_be_bytes());
        v
    }

    fn enc_u64_tagged(tag: u8, n: u64) -> Vec<u8> {
        let mut v = enc_tag(tag);
        v.extend_from_slice(&((n >> 32) as u32).to_be_bytes());
        v.extend_from_slice(&(n as u32).to_be_bytes());
        v
    }

    fn enc_int(n: u32) -> Vec<u8> {
        enc_u32_tagged(TAG_INT, n)
    }
    fn enc_ul(n: u32) -> Vec<u8> {
        enc_u32_tagged(TAG_UNSIGNED_LONG, n)
    }
    fn enc_ull(n: u64) -> Vec<u8> {
        enc_u64_tagged(TAG_UNSIGNED_LONG_LONG, n)
    }

    /// Build a mock stream with pre-loaded response bytes and a write-capture buffer.
    fn mock_stream(response: Vec<u8>) -> MarsStream<Cursor<Vec<u8>>, Vec<u8>> {
        MarsStream::new(Cursor::new(response), Vec::new())
    }

    // ── MarsStream protocol unit tests ────────────────────────────────────────

    #[test]
    fn test_read_string_basic() {
        let mut s = mock_stream(enc_string("hello"));
        assert_eq!(s.read_string().unwrap(), "hello");
    }

    #[test]
    fn test_read_string_empty() {
        let mut s = mock_stream(enc_string(""));
        assert_eq!(s.read_string().unwrap(), "");
    }

    #[test]
    fn test_write_string_encoding() {
        let mut s = mock_stream(vec![]);
        s.write_string("world").unwrap();
        assert_eq!(s.writer, enc_string("world"));
    }

    #[test]
    fn test_read_tag_skips_end_obj() {
        // Prepend two TAG_END_OBJ bytes before the actual UNSIGNED_LONG tag.
        let mut bytes = vec![TAG_END_OBJ, TAG_END_OBJ];
        bytes.extend_from_slice(&enc_ul(99));
        let mut s = mock_stream(bytes);
        assert_eq!(s.read_unsigned_long().unwrap(), 99);
    }

    #[test]
    fn test_read_int() {
        let mut s = mock_stream(enc_int(12345));
        assert_eq!(s.read_int().unwrap(), 12345);
    }

    #[test]
    fn test_read_unsigned_long() {
        let mut s = mock_stream(enc_ul(u32::MAX));
        assert_eq!(s.read_unsigned_long().unwrap(), u32::MAX);
    }

    #[test]
    fn test_read_unsigned_long_long() {
        let n: u64 = 0x0102_0304_0506_0708;
        let mut s = mock_stream(enc_ull(n));
        assert_eq!(s.read_unsigned_long_long().unwrap(), n);
    }

    #[test]
    fn test_write_object_encoding() {
        let mut s = mock_stream(vec![]);
        s.write_object("FetchAgent", "root_ref").unwrap();

        let mut expected = vec![TAG_START_OBJ];
        expected.extend_from_slice(&enc_string("FetchAgent"));
        expected.extend_from_slice(&enc_string("root_ref"));
        expected.push(TAG_END_OBJ);

        assert_eq!(s.writer, expected);
    }

    #[test]
    fn test_read_object_returns_class_name() {
        let mut bytes = vec![TAG_START_OBJ];
        bytes.extend_from_slice(&enc_string("PSimpleNode"));
        let mut s = mock_stream(bytes);
        assert_eq!(s.read_object().unwrap(), Some("PSimpleNode".to_string()));
    }

    #[test]
    fn test_read_object_returns_none_on_eof() {
        let mut s = mock_stream(vec![]);
        assert_eq!(s.read_object().unwrap(), None);
    }

    #[test]
    fn test_read_object_returns_none_on_null_byte() {
        let mut s = mock_stream(vec![0u8]);
        assert_eq!(s.read_object().unwrap(), None);
    }

    #[test]
    fn test_read_tag_unexpected_tag_returns_error() {
        // Feed TAG_STRING when TAG_INT is expected.
        let bytes = enc_string("oops");
        let mut s = mock_stream(bytes);
        assert!(s.read_int().is_err());
    }

    // ── decode_node_handler unit tests ────────────────────────────────────────

    fn build_simple_node_bytes(name: &str, children: &[(&str, &str)]) -> Vec<u8> {
        let mut b = enc_string(name);
        b.extend_from_slice(&enc_ul(children.len() as u32));
        for (v, r) in children {
            b.extend_from_slice(&enc_string(v));
            b.extend_from_slice(&enc_string(r));
        }
        b
    }

    #[test]
    fn test_decode_simple_node_two_children() {
        let bytes = build_simple_node_bytes("class", &[("od", "ref1"), ("rd", "ref2")]);
        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PSimpleNode", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Simple {
                name: "class".to_string(),
                children: vec![
                    ("od".to_string(), "ref1".to_string()),
                    ("rd".to_string(), "ref2".to_string()),
                ],
            })
        );
    }

    #[test]
    fn test_decode_balance_node_same_as_simple() {
        let bytes = build_simple_node_bytes("type", &[("fc", "ref_fc")]);
        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PBalanceNode", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Simple {
                name: "type".to_string(),
                children: vec![("fc".to_string(), "ref_fc".to_string())],
            })
        );
    }

    #[test]
    fn test_decode_simple_node_default_discards_default() {
        let mut bytes = enc_string("timespan");
        bytes.extend_from_slice(&enc_string("none")); // default value → discarded
        bytes.extend_from_slice(&enc_ul(1));
        bytes.extend_from_slice(&enc_string("instantaneous"));
        bytes.extend_from_slice(&enc_string("ref_inst"));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PSimpleNodeDefault", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Simple {
                name: "timespan".to_string(),
                children: vec![("instantaneous".to_string(), "ref_inst".to_string())],
            })
        );
    }

    #[test]
    fn test_decode_branch_node() {
        let mut bytes = enc_string("%param%==251");
        bytes.extend_from_slice(&enc_string("ref_true"));
        bytes.extend_from_slice(&enc_string("ref_false"));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PBranchNode", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Branch {
                expr: "%param%==251".to_string(),
                true_ref: "ref_true".to_string(),
                false_ref: "ref_false".to_string(),
            })
        );
    }

    #[test]
    fn test_decode_research_node_returns_expver_list() {
        let mut bytes = enc_string("expver");
        bytes.extend_from_slice(&enc_int(2));
        bytes.extend_from_slice(&enc_string("0001"));
        bytes.extend_from_slice(&enc_string("abcd"));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PResearchNode", &mut s, "").unwrap();

        // Verify the empty arg was written back to the server.
        assert_eq!(s.writer, enc_string(""));

        assert_eq!(
            result,
            Some(FetchedData::Research {
                name: "expver".to_string(),
                expvers: vec!["0001".to_string(), "abcd".to_string()],
            })
        );
    }

    #[test]
    fn test_decode_research_node_terminal() {
        let mut bytes = enc_string("expver");
        bytes.extend_from_slice(&enc_int(u32::MAX));
        bytes.extend_from_slice(&enc_string("next_ref_99"));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PResearchNode", &mut s, "0001").unwrap();

        // Verify "0001" was written to the server.
        assert_eq!(s.writer, enc_string("0001"));

        assert_eq!(
            result,
            Some(FetchedData::ResearchTerminal { next_ref: "next_ref_99".to_string() })
        );
    }

    #[test]
    fn test_decode_research_node_invalid_prefix() {
        let mut bytes = enc_string("expver");
        bytes.extend_from_slice(&enc_int(0)); // n == 0 → invalid

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PResearchNode", &mut s, "zzzz").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_leaf_node() {
        let mut bytes = enc_ull(98765);
        bytes.extend_from_slice(&enc_string("shape_ref_42"));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PLeafNode", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Redirect { shape_ref: "shape_ref_42".to_string() })
        );
    }

    #[test]
    fn test_decode_mono_axis_shape_single_axis() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&enc_ul(3));
        bytes.extend_from_slice(&enc_string("param"));
        bytes.extend_from_slice(&enc_string("130"));
        bytes.extend_from_slice(&enc_string("131"));
        bytes.extend_from_slice(&enc_string("132"));
        bytes.extend_from_slice(&enc_ul(0)); // terminator

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PMonoAxisShape", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Leaf {
                axes: vec![(
                    "param".to_string(),
                    vec!["130".to_string(), "131".to_string(), "132".to_string()],
                )],
            })
        );
    }

    #[test]
    fn test_decode_mono_axis_shape_two_axes() {
        let mut bytes = Vec::new();
        // axis: param
        bytes.extend_from_slice(&enc_ul(2));
        bytes.extend_from_slice(&enc_string("param"));
        bytes.extend_from_slice(&enc_string("130"));
        bytes.extend_from_slice(&enc_string("131"));
        // axis: step
        bytes.extend_from_slice(&enc_ul(3));
        bytes.extend_from_slice(&enc_string("step"));
        bytes.extend_from_slice(&enc_string("0"));
        bytes.extend_from_slice(&enc_string("6"));
        bytes.extend_from_slice(&enc_string("12"));
        // terminator
        bytes.extend_from_slice(&enc_ul(0));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PMonoAxisShape", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Leaf {
                axes: vec![
                    ("param".to_string(), vec!["130".to_string(), "131".to_string()]),
                    (
                        "step".to_string(),
                        vec!["0".to_string(), "6".to_string(), "12".to_string()],
                    ),
                ],
            })
        );
    }

    #[test]
    fn test_decode_bufr_shape_same_as_mono() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&enc_ul(1));
        bytes.extend_from_slice(&enc_string("obstype"));
        bytes.extend_from_slice(&enc_string("1"));
        bytes.extend_from_slice(&enc_ul(0));

        let mut s = mock_stream(bytes);
        let result = decode_node_handler("PBufrShape", &mut s, "").unwrap();
        assert_eq!(
            result,
            Some(FetchedData::Leaf {
                axes: vec![("obstype".to_string(), vec!["1".to_string()])],
            })
        );
    }

    #[test]
    fn test_decode_unknown_handler_returns_error() {
        let mut s = mock_stream(vec![]);
        let result = decode_node_handler("PGhostNode", &mut s, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown MARS node handler"));
    }

    // ── adduv unit tests ──────────────────────────────────────────────────────

    #[test]
    fn test_adduv_adds_u_v_when_vo_d_present() {
        let mut p = vec!["138".to_string(), "155".to_string(), "130".to_string()];
        adduv(&mut p);
        assert!(p.contains(&"131".to_string()), "131 should be added");
        assert!(p.contains(&"132".to_string()), "132 should be added");
    }

    #[test]
    fn test_adduv_no_op_when_u_v_already_present() {
        let mut p =
            vec!["138".to_string(), "155".to_string(), "131".to_string(), "132".to_string()];
        adduv(&mut p);
        assert_eq!(p.iter().filter(|x| *x == "131").count(), 1);
        assert_eq!(p.iter().filter(|x| *x == "132").count(), 1);
    }

    #[test]
    fn test_adduv_no_op_without_vo_d() {
        let mut p = vec!["130".to_string(), "131".to_string(), "132".to_string()];
        adduv(&mut p);
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn test_adduv_dotted_param_ids() {
        let mut p = vec!["138.128".to_string(), "155.128".to_string()];
        adduv(&mut p);
        assert!(p.contains(&"131".to_string()));
        assert!(p.contains(&"132".to_string()));
    }

    #[test]
    fn test_adduv_only_vor_no_action() {
        // Only VOR present, not DIV → no insertion.
        let mut p = vec!["138".to_string(), "130".to_string()];
        adduv(&mut p);
        assert_eq!(p.len(), 2);
    }

    // ── make_coords unit tests ────────────────────────────────────────────────

    #[test]
    fn test_make_coords_integer() {
        let c = make_coords(&["42"]).unwrap();
        assert!(matches!(c, Coordinates::Integers(_)));
    }

    #[test]
    fn test_make_coords_float() {
        let c = make_coords(&["3.14"]).unwrap();
        assert!(matches!(c, Coordinates::Floats(_)));
    }

    #[test]
    fn test_make_coords_string() {
        let c = make_coords(&["od"]).unwrap();
        assert!(matches!(c, Coordinates::Strings(_)));
    }

    #[test]
    fn test_make_coords_leading_zero_stays_string() {
        // "0001" looks like an integer but must be preserved as a string.
        let c = make_coords(&["0001"]).unwrap();
        assert!(matches!(c, Coordinates::Strings(_)));
    }

    #[test]
    fn test_make_coords_empty_inputs_return_none() {
        assert!(make_coords(&[]).is_none());
        assert!(make_coords(&[""]).is_none());
        assert!(make_coords(&["  "]).is_none());
    }

    #[test]
    fn test_make_coords_multiple_integers() {
        let c = make_coords(&["130", "131", "132"]).unwrap();
        assert!(matches!(c, Coordinates::Integers(_)));
    }

    #[test]
    fn test_make_coords_negative_integer() {
        let c = make_coords(&["-1"]).unwrap();
        assert!(matches!(c, Coordinates::Integers(_)));
    }

    #[test]
    fn test_make_coords_zero() {
        // "0" has no leading-zero problem (single digit), should be an integer.
        let c = make_coords(&["0"]).unwrap();
        assert!(matches!(c, Coordinates::Integers(_)));
    }

    #[test]
    fn test_make_coords_dotted_float_param() {
        // "130.128" is a MARS param id encoded as a float.
        let c = make_coords(&["130.128"]).unwrap();
        assert!(matches!(c, Coordinates::Floats(_)));
    }

    // ── Integration test (requires a live MARS catalogue server) ─────────────

    #[test]
    #[ignore = "requires live MARS catalogue server; set MARS_CATALOGUE_HOST and MARS_CATALOGUE_PORT"]
    fn test_from_mars_server_integration() {
        let host = std::env::var("MARS_CATALOGUE_HOST")
            .expect("set MARS_CATALOGUE_HOST to run this test");
        let port: u16 = std::env::var("MARS_CATALOGUE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .expect("set MARS_CATALOGUE_PORT to run this test");

        let qube = <Qube as FromMarsServer>::from_mars_server(&host, port)
            .expect("failed to build Qube from MARS server");
        let ascii = qube.to_ascii();
        println!("{ascii}");
        assert!(!ascii.is_empty());
    }
}
