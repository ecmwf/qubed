//! Adapter that builds a [`Qube`] by exhaustively traversing a live MARS
//! catalogue server.
//!
//! Every node is fetched over its own TCP connection as one eckit `Stream`
//! exchange: the client sends a `FetchAgent` object carrying the node ref,
//! the server answers with a password request (ignored) followed by the node
//! object, or closes the connection when the ref is not in the catalogue.
//!
//! # Node types
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

use std::thread;
use std::time::Duration;

use eckit::{Stream, TcpStream};
use qubed::{Coordinates, NodeIdx, Qube};

/// Sent by a `PResearchNode` as the match count once the accumulated prefix
/// uniquely resolves one experiment version; the next node ref follows.
const RESEARCH_TERMINAL: i32 = -1;

/// Decoded payload of a single MARS catalogue node.
#[derive(Debug, Clone, PartialEq)]
enum FetchedData {
    /// `PSimpleNode` / `PSimpleNodeDefault` / `PBalanceNode`: a dimension with
    /// `(value, child_ref)` pairs in wire order.
    Simple { name: String, children: Vec<(String, String)> },

    /// `PBranchNode`: conditional routing. Both branches are followed under
    /// the same parent so that `compress()` can merge the parallel subtrees.
    Branch { true_ref: String, false_ref: String },

    /// `PResearchNode` with a positive count: the experiment versions
    /// available at this node. Each is resolved to its terminal ref by
    /// [`traverse_research_expver`].
    Research { name: String, expvers: Vec<String> },

    /// `PResearchNode` with [`RESEARCH_TERMINAL`]: the prefix resolved and the
    /// server sent the ref of the next node.
    ResearchTerminal { next_ref: String },

    /// `PLeafNode`: a forwarding pointer to a shape node.
    Redirect { shape_ref: String },

    /// `PMonoAxisShape` / `PBufrShape` / `PShape`: `(axis_name, values)` pairs
    /// in wire order.
    Leaf { axes: Vec<(String, Vec<String>)> },
}

/// If `params` contains VOR (`138`) and DIV (`155`) but not U (`131`) and
/// V (`132`), append `"131"` and `"132"`.
fn adduv(params: &mut Vec<String>) {
    let has = |p: &str| params.iter().any(|x| x == p);
    let has_vo_d = (has("138") && has("155")) || (has("138.128") && has("155.128"));
    let has_u_v = (has("131") && has("132")) || (has("131.128") && has("132.128"));
    if has_vo_d && !has_u_v {
        params.push("131".to_string());
        params.push("132".to_string());
    }
}

fn coords(values: &[String]) -> Option<Coordinates> {
    let coords = Coordinates::from_string(&values.join("/"));
    if coords.is_empty() { None } else { Some(coords) }
}

fn read_children(stream: &mut dyn Stream) -> eckit::Result<Vec<(String, String)>> {
    let count = stream.read_u32()?;
    (0..count).map(|_| Ok((stream.read_string()?, stream.read_string()?))).collect()
}

/// Decode the node payload that follows the handler class name on `stream`.
///
/// `arg` is the experiment-version prefix a `PResearchNode` sends back to the
/// server; it is empty for the initial enumeration and for every other node.
fn decode_node_handler(
    handler: &str,
    stream: &mut dyn Stream,
    arg: &str,
) -> Result<Option<FetchedData>, String> {
    let io = |e: eckit::Error| format!("{handler}: {e}");

    match handler {
        "PSimpleNode" | "PBalanceNode" => {
            let name = stream.read_string().map_err(io)?;
            let children = read_children(stream).map_err(io)?;
            Ok(Some(FetchedData::Simple { name, children }))
        }

        "PSimpleNodeDefault" => {
            let name = stream.read_string().map_err(io)?;
            stream.read_string().map_err(io)?;
            let children = read_children(stream).map_err(io)?;
            Ok(Some(FetchedData::Simple { name, children }))
        }

        "PBranchNode" => {
            stream.read_string().map_err(io)?;
            let true_ref = stream.read_string().map_err(io)?;
            let false_ref = stream.read_string().map_err(io)?;
            Ok(Some(FetchedData::Branch { true_ref, false_ref }))
        }

        "PResearchNode" => {
            let name = stream.read_string().map_err(io)?;
            stream.write_string(arg).map_err(io)?;
            match stream.read_i32().map_err(io)? {
                0 => Ok(None),
                RESEARCH_TERMINAL => {
                    let next_ref = stream.read_string().map_err(io)?;
                    Ok(Some(FetchedData::ResearchTerminal { next_ref }))
                }
                n => {
                    let expvers = (0..n)
                        .map(|_| stream.read_string())
                        .collect::<eckit::Result<_>>()
                        .map_err(io)?;
                    Ok(Some(FetchedData::Research { name, expvers }))
                }
            }
        }

        "PLeafNode" => {
            stream.read_u64().map_err(io)?;
            let shape_ref = stream.read_string().map_err(io)?;
            Ok(Some(FetchedData::Redirect { shape_ref }))
        }

        "PMonoAxisShape" | "PBufrShape" | "PShape" => {
            let mut axes = Vec::new();
            loop {
                let count = stream.read_u32().map_err(io)?;
                if count == 0 {
                    break;
                }
                let name = stream.read_string().map_err(io)?;
                let mut values = (0..count)
                    .map(|_| stream.read_string())
                    .collect::<eckit::Result<Vec<_>>>()
                    .map_err(io)?;
                if name == "param" {
                    adduv(&mut values);
                }
                axes.push((name, values));
            }
            Ok(Some(FetchedData::Leaf { axes }))
        }

        other => Err(format!("Unknown MARS node handler: '{other}'")),
    }
}

trait NodeFetcher {
    fn fetch(&self, ref_: &str, arg: &str) -> Result<Option<FetchedData>, String>;
}

/// A MARS catalogue server to traverse.
#[derive(Debug, Clone)]
pub struct MarsServer {
    host: String,
    port: u16,
    retries: u32,
    backoff: Duration,
}

impl MarsServer {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self { host: host.into(), port, retries: 3, backoff: Duration::from_secs(1) }
    }

    /// Further attempts per node fetch after the first one fails. Default 3.
    /// Connecting already retries inside eckit (five tries, five seconds
    /// apart), so this mainly covers exchanges that fail after connecting.
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Delay before the first retry, doubled on every further one. Default 1s.
    pub fn backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    fn fetch_once(&self, ref_: &str, arg: &str) -> Result<Option<FetchedData>, String> {
        let mut stream = TcpStream::connect(&self.host, i32::from(self.port))
            .map_err(|e| format!("connect to {}:{}: {e}", self.host, self.port))?;
        let io = |e: eckit::Error| format!("FetchAgent {ref_:?}: {e}");

        stream.start_object().map_err(io)?;
        stream.write_string("FetchAgent").map_err(io)?;
        stream.write_string(ref_).map_err(io)?;
        stream.end_object().map_err(io)?;

        stream.read_i32().map_err(io)?;
        if !stream.next_object().map_err(io)? {
            return Ok(None);
        }
        let handler = stream.read_string().map_err(io)?;
        decode_node_handler(&handler, &mut stream, arg)
    }
}

impl NodeFetcher for MarsServer {
    fn fetch(&self, ref_: &str, arg: &str) -> Result<Option<FetchedData>, String> {
        let mut delay = self.backoff;
        let mut failures = 0;
        loop {
            match self.fetch_once(ref_, arg) {
                Err(_) if failures < self.retries => {
                    failures += 1;
                    thread::sleep(delay);
                    delay *= 2;
                }
                result => return result,
            }
        }
    }
}

/// Resolve `expver` to the ref of the node below it by sending the server one
/// more character of the version at a time until it answers with a terminal.
///
/// Returns `None` if the server declares the prefix invalid at any point.
fn traverse_research_expver(
    fetcher: &dyn NodeFetcher,
    research_ref: &str,
    expver: &str,
) -> Result<Option<String>, String> {
    let mut prefix = String::with_capacity(expver.len());
    for ch in expver.chars() {
        prefix.push(ch);
        match fetcher.fetch(research_ref, &prefix)? {
            None => return Ok(None),
            Some(FetchedData::ResearchTerminal { next_ref }) => return Ok(Some(next_ref)),
            Some(FetchedData::Research { .. }) => {}
            Some(other) => {
                return Err(format!(
                    "Unexpected node type during research traversal for expver '{expver}': {other:?}"
                ));
            }
        }
    }
    Ok(None)
}

fn build_subtree(
    fetcher: &dyn NodeFetcher,
    ref_: &str,
    arg: &str,
    qube: &mut Qube,
    parent: NodeIdx,
) -> Result<(), String> {
    let Some(data) = fetcher.fetch(ref_, arg)? else {
        return Ok(());
    };

    match data {
        FetchedData::Simple { name, children } => {
            for (value, child_ref) in children {
                let child = qube
                    .get_or_create_child(&name, parent, coords(std::slice::from_ref(&value)))
                    .map_err(|e| format!("get_or_create_child({name}={value}): {e:?}"))?;
                build_subtree(fetcher, &child_ref, "", qube, child)?;
            }
        }

        FetchedData::Branch { true_ref, false_ref } => {
            if !true_ref.is_empty() {
                build_subtree(fetcher, &true_ref, "", qube, parent)?;
            }
            if !false_ref.is_empty() {
                build_subtree(fetcher, &false_ref, "", qube, parent)?;
            }
        }

        FetchedData::Research { name, expvers } => {
            for expver in expvers {
                let child = qube
                    .get_or_create_child(&name, parent, coords(std::slice::from_ref(&expver)))
                    .map_err(|e| format!("get_or_create_child({name}={expver}): {e:?}"))?;
                if let Some(next_ref) = traverse_research_expver(fetcher, ref_, &expver)? {
                    build_subtree(fetcher, &next_ref, "", qube, child)?;
                }
            }
        }

        FetchedData::ResearchTerminal { next_ref } => {
            build_subtree(fetcher, &next_ref, "", qube, parent)?;
        }

        FetchedData::Redirect { shape_ref } => {
            build_subtree(fetcher, &shape_ref, "", qube, parent)?;
        }

        FetchedData::Leaf { axes } => {
            let mut current = parent;
            for (name, values) in axes {
                current = qube
                    .get_or_create_child(&name, current, coords(&values))
                    .map_err(|e| format!("get_or_create_child({name}): {e:?}"))?;
            }
        }
    }

    Ok(())
}

fn build_qube(fetcher: &dyn NodeFetcher) -> Result<Qube, String> {
    let mut qube = Qube::new();
    let root = qube.root();
    build_subtree(fetcher, "", "", &mut qube, root)?;
    qube.compress();
    Ok(qube)
}

/// Build a [`Qube`] by exhaustively traversing a live MARS catalogue server.
///
/// All dimension names and values are discovered from the server; no query
/// filter is applied.
pub trait FromMarsServer {
    fn from_mars_server(host: &str, port: u16) -> Result<Qube, String>;
    fn from_mars_server_with(server: &MarsServer) -> Result<Qube, String>;
}

impl FromMarsServer for Qube {
    fn from_mars_server(host: &str, port: u16) -> Result<Qube, String> {
        Self::from_mars_server_with(&MarsServer::new(host, port))
    }

    fn from_mars_server_with(server: &MarsServer) -> Result<Qube, String> {
        eckit::init();
        build_qube(server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eckit::MemoryStream;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    type Fetched = Result<Option<FetchedData>, String>;

    fn written(write: impl FnOnce(&mut dyn Stream) -> eckit::Result<()>) -> Vec<u8> {
        eckit::init();
        let mut writer = MemoryStream::writer();
        write(&mut writer).unwrap();
        writer.buffer().unwrap().to_vec()
    }

    fn decode(handler: &str, bytes: &[u8]) -> Fetched {
        let mut reader = MemoryStream::reader(bytes);
        decode_node_handler(handler, &mut reader, "")
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    fn simple_node(name: &str, children: &[(&str, &str)]) -> Vec<u8> {
        written(|s| {
            s.write_string(name)?;
            s.write_u32(children.len() as u32)?;
            for (value, child_ref) in children {
                s.write_string(value)?;
                s.write_string(child_ref)?;
            }
            Ok(())
        })
    }

    fn shape(axes: &[(&str, &[&str])]) -> Vec<u8> {
        written(|s| {
            for (name, values) in axes {
                s.write_u32(values.len() as u32)?;
                s.write_string(name)?;
                for value in *values {
                    s.write_string(value)?;
                }
            }
            s.write_u32(0)
        })
    }

    #[test]
    fn decodes_simple_node() {
        let bytes = simple_node("class", &[("od", "ref1"), ("rd", "ref2")]);
        assert_eq!(
            decode("PSimpleNode", &bytes).unwrap(),
            Some(FetchedData::Simple {
                name: "class".into(),
                children: pairs(&[("od", "ref1"), ("rd", "ref2")]),
            })
        );
    }

    #[test]
    fn decodes_balance_node_like_a_simple_node() {
        let bytes = simple_node("type", &[("fc", "ref_fc")]);
        assert_eq!(
            decode("PBalanceNode", &bytes).unwrap(),
            Some(FetchedData::Simple { name: "type".into(), children: pairs(&[("fc", "ref_fc")]) })
        );
    }

    #[test]
    fn decodes_simple_node_default_and_drops_the_default() {
        let bytes = written(|s| {
            s.write_string("timespan")?;
            s.write_string("none")?;
            s.write_u32(1)?;
            s.write_string("instantaneous")?;
            s.write_string("ref_inst")
        });
        assert_eq!(
            decode("PSimpleNodeDefault", &bytes).unwrap(),
            Some(FetchedData::Simple {
                name: "timespan".into(),
                children: pairs(&[("instantaneous", "ref_inst")]),
            })
        );
    }

    #[test]
    fn decodes_branch_node() {
        let bytes = written(|s| {
            s.write_string("%param%==251")?;
            s.write_string("ref_true")?;
            s.write_string("ref_false")
        });
        assert_eq!(
            decode("PBranchNode", &bytes).unwrap(),
            Some(FetchedData::Branch {
                true_ref: "ref_true".into(),
                false_ref: "ref_false".into()
            })
        );
    }

    #[test]
    fn decodes_leaf_node_as_redirect() {
        let bytes = written(|s| {
            s.write_u64(98765)?;
            s.write_string("shape_ref_42")
        });
        assert_eq!(
            decode("PLeafNode", &bytes).unwrap(),
            Some(FetchedData::Redirect { shape_ref: "shape_ref_42".into() })
        );
    }

    #[test]
    fn decodes_mono_axis_shape_with_two_axes() {
        let bytes = shape(&[("param", &["130", "131"]), ("step", &["0", "6", "12"])]);
        assert_eq!(
            decode("PMonoAxisShape", &bytes).unwrap(),
            Some(FetchedData::Leaf {
                axes: vec![
                    ("param".into(), strings(&["130", "131"])),
                    ("step".into(), strings(&["0", "6", "12"])),
                ],
            })
        );
    }

    #[test]
    fn decodes_bufr_shape_like_mono_axis() {
        let bytes = shape(&[("obstype", &["1"])]);
        assert_eq!(
            decode("PBufrShape", &bytes).unwrap(),
            Some(FetchedData::Leaf { axes: vec![("obstype".into(), strings(&["1"]))] })
        );
    }

    #[test]
    fn shape_param_axis_gains_u_and_v_from_vorticity_and_divergence() {
        let bytes = shape(&[("param", &["138", "155"])]);
        assert_eq!(
            decode("PShape", &bytes).unwrap(),
            Some(FetchedData::Leaf {
                axes: vec![("param".into(), strings(&["138", "155", "131", "132"]))],
            })
        );
    }

    #[test]
    fn rejects_unknown_handler() {
        let err = decode("PGhostNode", &[]).unwrap_err();
        assert!(err.contains("Unknown MARS node handler"), "{err}");
    }

    #[test]
    fn adduv_is_a_no_op_when_u_and_v_are_present_or_vorticity_is_alone() {
        let mut present = strings(&["138", "155", "131", "132"]);
        adduv(&mut present);
        assert_eq!(present, strings(&["138", "155", "131", "132"]));

        let mut alone = strings(&["138", "130"]);
        adduv(&mut alone);
        assert_eq!(alone, strings(&["138", "130"]));
    }

    #[test]
    fn adduv_handles_dotted_param_ids() {
        let mut params = strings(&["138.128", "155.128"]);
        adduv(&mut params);
        assert_eq!(params, strings(&["138.128", "155.128", "131", "132"]));
    }

    #[test]
    fn coords_keep_leading_zeros_as_strings_and_parse_numbers() {
        assert!(matches!(coords(&strings(&["0001"])), Some(Coordinates::Strings(_))));
        assert!(matches!(coords(&strings(&["130", "131"])), Some(Coordinates::Integers(_))));
        assert!(matches!(coords(&strings(&["130.128"])), Some(Coordinates::Floats(_))));
        assert!(coords(&[]).is_none());
    }

    struct Catalogue(HashMap<(String, String), Fetched>);

    impl Catalogue {
        fn new(entries: Vec<((&str, &str), Fetched)>) -> Self {
            Self(
                entries
                    .into_iter()
                    .map(|((r, a), v)| ((r.to_string(), a.to_string()), v))
                    .collect(),
            )
        }
    }

    impl NodeFetcher for Catalogue {
        fn fetch(&self, ref_: &str, arg: &str) -> Result<Option<FetchedData>, String> {
            self.0.get(&(ref_.to_string(), arg.to_string())).cloned().unwrap_or(Ok(None))
        }
    }

    fn simple(name: &str, children: &[(&str, &str)]) -> Fetched {
        Ok(Some(FetchedData::Simple { name: name.into(), children: pairs(children) }))
    }

    fn leaf(axes: &[(&str, &[&str])]) -> Fetched {
        Ok(Some(FetchedData::Leaf {
            axes: axes.iter().map(|(n, v)| (n.to_string(), strings(v))).collect(),
        }))
    }

    fn research(expvers: &[&str]) -> Fetched {
        Ok(Some(FetchedData::Research { name: "expver".into(), expvers: strings(expvers) }))
    }

    fn terminal(next_ref: &str) -> Fetched {
        Ok(Some(FetchedData::ResearchTerminal { next_ref: next_ref.into() }))
    }

    #[test]
    fn builds_a_qube_across_simple_branch_redirect_research_and_leaf_nodes() {
        let catalogue = Catalogue::new(vec![
            (("", ""), simple("class", &[("od", "c-od"), ("rd", "c-rd")])),
            (("c-od", ""), simple("stream", &[("oper", "s-oper")])),
            (
                ("s-oper", ""),
                Ok(Some(FetchedData::Branch {
                    true_ref: "b-true".into(),
                    false_ref: "b-absent".into(),
                })),
            ),
            (("b-true", ""), Ok(Some(FetchedData::Redirect { shape_ref: "shape-1".into() }))),
            (("shape-1", ""), leaf(&[("param", &["130", "131"]), ("step", &["0", "6"])])),
            (("c-rd", ""), research(&["0001", "abcd"])),
            (("c-rd", "0"), research(&["0001"])),
            (("c-rd", "00"), research(&["0001"])),
            (("c-rd", "000"), research(&["0001"])),
            (("c-rd", "0001"), terminal("e-0001")),
            (("c-rd", "a"), terminal("e-abcd")),
            (("e-0001", ""), leaf(&[("param", &["130"])])),
            (("e-abcd", ""), leaf(&[("param", &["130"])])),
        ]);

        let qube = build_qube(&catalogue).unwrap();
        let datacubes = qube.to_datacubes();
        assert_eq!(datacubes.len(), 2, "{}", qube.to_ascii());

        let by_class = |class: &str| {
            datacubes
                .iter()
                .map(|dc| dc.coordinates())
                .find(|c| c["class"].iter_sorted_strings() == vec![class])
                .unwrap_or_else(|| panic!("no datacube for class={class}"))
        };

        let od = by_class("od");
        assert_eq!(od["stream"].iter_sorted_strings(), vec!["oper"]);
        assert_eq!(od["param"].iter_sorted_strings(), vec!["130", "131"]);
        assert_eq!(od["step"].iter_sorted_strings(), vec!["0", "6"]);

        let rd = by_class("rd");
        assert_eq!(rd["expver"].iter_sorted_strings(), vec!["0001", "abcd"]);
        assert_eq!(rd["param"].iter_sorted_strings(), vec!["130"]);
    }

    #[test]
    fn absent_root_gives_an_empty_qube() {
        let qube = build_qube(&Catalogue::new(vec![])).unwrap();
        assert!(qube.is_empty());
    }

    #[test]
    fn a_failed_fetch_aborts_the_build() {
        let catalogue = Catalogue::new(vec![
            (("", ""), simple("class", &[("od", "c-od")])),
            (("c-od", ""), Err("connection reset".into())),
        ]);
        let err = build_qube(&catalogue).unwrap_err();
        assert!(err.contains("connection reset"), "{err}");
    }

    #[test]
    fn expver_that_never_resolves_gets_no_subtree() {
        let catalogue = Catalogue::new(vec![
            (("", ""), research(&["zz"])),
            (("", "z"), research(&["zz"])),
            (("", "zz"), research(&["zz"])),
        ]);
        let qube = build_qube(&catalogue).unwrap();
        let datacubes = qube.to_datacubes();
        assert_eq!(datacubes.len(), 1);
        assert_eq!(datacubes[0].coordinates().len(), 1);
    }

    fn request_bytes(ref_: &str) -> Vec<u8> {
        written(|s| {
            s.start_object()?;
            s.write_string("FetchAgent")?;
            s.write_string(ref_)?;
            s.end_object()
        })
    }

    /// Accept one connection, hand the request bytes back, then run `respond`.
    fn serve_once(
        request_len: usize,
        respond: impl FnOnce(&mut std::net::TcpStream) + Send + 'static,
    ) -> (u16, mpsc::Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut request = vec![0u8; request_len];
            conn.read_exact(&mut request).unwrap();
            tx.send(request).unwrap();
            respond(&mut conn);
        });
        (port, rx)
    }

    fn read_tagged_string(conn: &mut std::net::TcpStream) -> String {
        let mut header = [0u8; 5];
        conn.read_exact(&mut header).unwrap();
        let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
        let mut bytes = vec![0u8; len];
        conn.read_exact(&mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn fetches_a_node_over_tcp() {
        eckit::init();
        let request = request_bytes("");
        let response = written(|s| {
            s.write_i32(0)?;
            s.start_object()?;
            s.write_string("PSimpleNode")?;
            s.write_string("class")?;
            s.write_u32(1)?;
            s.write_string("od")?;
            s.write_string("ref-od")
        });
        let (port, rx) = serve_once(request.len(), move |conn| conn.write_all(&response).unwrap());

        let fetched = MarsServer::new("127.0.0.1", port).retries(0).fetch("", "").unwrap();

        assert_eq!(rx.recv().unwrap(), request);
        assert_eq!(
            fetched,
            Some(FetchedData::Simple {
                name: "class".into(),
                children: pairs(&[("od", "ref-od")])
            })
        );
    }

    #[test]
    fn a_closed_connection_after_the_password_request_means_absent() {
        eckit::init();
        let request = request_bytes("missing");
        let response = written(|s| s.write_i32(0));
        let (port, _rx) = serve_once(request.len(), move |conn| conn.write_all(&response).unwrap());

        let fetched = MarsServer::new("127.0.0.1", port).retries(0).fetch("missing", "").unwrap();
        assert_eq!(fetched, None);
    }

    #[test]
    fn research_node_sends_the_prefix_back_before_reading_the_answer() {
        eckit::init();
        let request = request_bytes("r");
        let head = written(|s| {
            s.write_i32(0)?;
            s.start_object()?;
            s.write_string("PResearchNode")?;
            s.write_string("expver")
        });
        let tail = written(|s| {
            s.write_i32(RESEARCH_TERMINAL)?;
            s.write_string("next-ref")
        });
        let (prefix_tx, prefix_rx) = mpsc::channel();
        let (port, _rx) = serve_once(request.len(), move |conn| {
            conn.write_all(&head).unwrap();
            prefix_tx.send(read_tagged_string(conn)).unwrap();
            conn.write_all(&tail).unwrap();
        });

        let fetched = MarsServer::new("127.0.0.1", port).retries(0).fetch("r", "0001").unwrap();

        assert_eq!(prefix_rx.recv().unwrap(), "0001");
        assert_eq!(fetched, Some(FetchedData::ResearchTerminal { next_ref: "next-ref".into() }));
    }

    #[test]
    #[ignore = "requires a live MARS catalogue server; set MARS_CATALOGUE_HOST and MARS_CATALOGUE_PORT"]
    fn traverses_a_live_catalogue() {
        let host =
            std::env::var("MARS_CATALOGUE_HOST").expect("set MARS_CATALOGUE_HOST to run this test");
        let port: u16 = std::env::var("MARS_CATALOGUE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .expect("set MARS_CATALOGUE_PORT to run this test");

        let qube = Qube::from_mars_server(&host, port).expect("traverse MARS catalogue");
        println!("{}", qube.to_ascii());
        assert!(!qube.is_empty());
    }
}
