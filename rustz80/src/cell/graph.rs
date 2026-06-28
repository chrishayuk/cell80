//! `CellGraph` — static, **host-routed** composition of cells.
//!
//! A graph names cells (`nodes`), wires each node's input ports from constants / external
//! inputs / another node's output (`wires`), and exposes chosen node outputs (`outputs`). The
//! **host** routes typed values between cells; the cells never see each other — no sockets, no
//! shared memory, no ambient authority. Each cell is still the same bounded, deterministic
//! sandbox; the graph just decides who feeds whom.
//!
//! The unique win is that the artifacts are *typed*, so a graph is **validated before a single
//! cycle runs**: every wire's source-port type must match its destination-port type, every
//! value-cell input must be fed, and the graph must be acyclic. A graph that wouldn't type is
//! rejected up front, not discovered mid-run.
//!
//! Port model (derived from each cell's manifest signature):
//! * **input ports** — a value cell's `params`, or a state cell's `state` fields.
//! * **output ports** — `result` (the return value), plus a state cell's `state` fields.
use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// A reference to a node's port, rendered `node.port`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Port {
    pub node: String,
    pub port: String,
}

impl Port {
    pub fn new(node: impl Into<String>, port: impl Into<String>) -> Self {
        Self {
            node: node.into(),
            port: port.into(),
        }
    }
    fn key(&self) -> String {
        format!("{}.{}", self.node, self.port)
    }
}

/// Where a node input port draws its value.
#[derive(Clone, Debug)]
pub enum Feed {
    /// Another node's output port (an inter-cell edge).
    From(Port),
    /// A literal constant.
    Const(u64),
    /// An external graph input, by name.
    Input(String),
}

/// A static graph of cells wired host-side.
pub struct CellGraph {
    pub id: String,
    /// `(node id, cell id)` — a node names a cell in the host catalog.
    pub nodes: Vec<(String, String)>,
    /// `(destination input port, where it is fed from)`.
    pub wires: Vec<(Port, Feed)>,
    /// `(graph output name, the node output port it exposes)`.
    pub outputs: Vec<(String, Port)>,
}

/// One node's execution record — the per-step provenance of a graph run.
#[derive(Debug, Clone)]
pub struct NodeTrace {
    pub node: String,
    pub cell: String,
    pub inputs: Vec<(String, u64)>,
    pub result: u16,
    pub cycles: u64,
    pub trapped_ops: u64,
}

/// The result of running a graph: the named outputs, the full per-node trace, and the summed
/// cost across all nodes.
#[derive(Debug, Clone)]
pub struct GraphRun {
    pub outputs: Vec<(String, u64)>,
    pub trace: Vec<NodeTrace>,
    pub cycles: u64,
    pub trapped_ops: u64,
}

/// A node's input ports `(name, type)` — state fields for a state cell, else value params.
fn input_ports(m: &Manifest) -> Vec<(String, String)> {
    if m.signature.state.is_empty() {
        m.signature.params.clone()
    } else {
        m.signature.state.clone()
    }
}

/// A node's output ports `(name, type)` — `result` (the return) plus any state fields.
fn output_ports(m: &Manifest) -> Vec<(String, String)> {
    let mut v = vec![("result".to_string(), m.signature.ret.clone())];
    v.extend(m.signature.state.iter().cloned());
    v
}

fn port_type(ports: &[(String, String)], name: &str) -> Option<String> {
    ports
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, t)| t.clone())
}

impl CellGraph {
    fn node_cell(&self, node: &str) -> Option<&str> {
        self.nodes
            .iter()
            .find(|(n, _)| n == node)
            .map(|(_, c)| c.as_str())
    }

    fn manifest<'h>(&self, host: &'h CellHost, node: &str) -> Result<&'h Manifest, String> {
        let cell = self
            .node_cell(node)
            .ok_or_else(|| format!("no node `{node}`"))?;
        host.manifest(cell)
            .ok_or_else(|| format!("node `{node}`: no cell `{cell}`"))
    }

    /// Type-check and structurally validate the graph against `host` — **before** running:
    /// nodes/cells exist, every wire connects real ports of matching type and feeds each input
    /// at most once, every value-cell param is fed, outputs name real output ports, and the
    /// graph is acyclic. Returns the first problem found.
    pub fn validate(&self, host: &CellHost) -> Result<(), String> {
        // Nodes: unique ids, known cells.
        let mut seen = HashSet::new();
        for (node, cell) in &self.nodes {
            if !seen.insert(node.as_str()) {
                return Err(format!("duplicate node `{node}`"));
            }
            if host.manifest(cell).is_none() {
                return Err(format!("node `{node}`: no cell `{cell}`"));
            }
        }

        // Wires: real ports, matching types, fed at most once.
        let mut fed: HashSet<String> = HashSet::new();
        for (dest, feed) in &self.wires {
            let dtype = port_type(&input_ports(self.manifest(host, &dest.node)?), &dest.port)
                .ok_or_else(|| format!("node `{}` has no input port `{}`", dest.node, dest.port))?;
            if !fed.insert(dest.key()) {
                return Err(format!("input `{}` is fed more than once", dest.key()));
            }
            let stype = match feed {
                Feed::From(src) => {
                    port_type(&output_ports(self.manifest(host, &src.node)?), &src.port)
                        .ok_or_else(|| {
                            format!("node `{}` has no output port `{}`", src.node, src.port)
                        })?
                }
                // Constants and external inputs are u16-wide (the addressable slot).
                Feed::Const(_) | Feed::Input(_) => "u16".to_string(),
            };
            if stype != dtype {
                return Err(format!(
                    "type mismatch at `{}`: source is `{stype}`, port is `{dtype}`",
                    dest.key()
                ));
            }
        }

        // Value cells: every param must be fed (a missing arg is undefined).
        for (node, _) in &self.nodes {
            let m = self.manifest(host, node)?;
            if m.signature.state.is_empty() {
                for (p, _) in &m.signature.params {
                    if !fed.contains(&format!("{node}.{p}")) {
                        return Err(format!("node `{node}` input `{p}` is not wired"));
                    }
                }
            }
        }

        // Outputs: real output ports.
        for (name, port) in &self.outputs {
            let m = self.manifest(host, &port.node)?;
            if port_type(&output_ports(m), &port.port).is_none() {
                return Err(format!(
                    "output `{name}`: node `{}` has no output port `{}`",
                    port.node, port.port
                ));
            }
        }

        self.topo_order()?; // acyclic
        Ok(())
    }

    /// A deterministic topological order of node ids (Kahn's algorithm over `From` wires),
    /// preserving declaration order among ready nodes. Errors if the graph has a cycle.
    fn topo_order(&self) -> Result<Vec<String>, String> {
        let mut indeg: HashMap<&str, usize> =
            self.nodes.iter().map(|(n, _)| (n.as_str(), 0)).collect();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for (dest, feed) in &self.wires {
            if let Feed::From(src) = feed {
                adj.entry(src.node.as_str())
                    .or_default()
                    .push(dest.node.as_str());
                *indeg
                    .get_mut(dest.node.as_str())
                    .ok_or_else(|| format!("wire to unknown node `{}`", dest.node))? += 1;
            }
        }
        let mut q: VecDeque<&str> = self
            .nodes
            .iter()
            .map(|(n, _)| n.as_str())
            .filter(|n| indeg[n] == 0)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(n) = q.pop_front() {
            order.push(n.to_string());
            for &s in adj.get(n).into_iter().flatten() {
                let d = indeg.get_mut(s).unwrap();
                *d -= 1;
                if *d == 0 {
                    q.push_back(s);
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err("graph has a cycle".into());
        }
        Ok(order)
    }

    /// Validate, then execute: run each node in topological order on a warm runner from
    /// `host`, routing typed values along the wires, and return the named outputs plus a full
    /// per-node trace. `inputs` supplies the external graph inputs by name.
    pub fn run(
        &self,
        host: &mut CellHost,
        inputs: &HashMap<String, u64>,
        budget: u64,
    ) -> Result<GraphRun, String> {
        self.validate(host)?;
        let order = self.topo_order()?;

        let mut values: HashMap<String, u64> = HashMap::new(); // "node.port" -> value
        let mut trace = Vec::with_capacity(order.len());
        let (mut total_cyc, mut total_trap) = (0u64, 0u64);

        for node in &order {
            let cell = self.node_cell(node).unwrap().to_string();
            let m = host.manifest(&cell).unwrap().clone();

            // Resolve this node's wired inputs (port name -> value).
            let mut resolved: HashMap<String, u64> = HashMap::new();
            for (dest, feed) in self.wires.iter().filter(|(d, _)| d.node == *node) {
                let v = match feed {
                    Feed::Const(c) => *c,
                    Feed::Input(name) => *inputs
                        .get(name)
                        .ok_or_else(|| format!("missing graph input `{name}`"))?,
                    Feed::From(src) => *values
                        .get(&src.key())
                        .ok_or_else(|| format!("unresolved `{}` (topology bug)", src.key()))?,
                };
                resolved.insert(dest.port.clone(), v);
            }

            // Run the node — value cell by positional args, state cell by named fields.
            let handle = host.load(&cell)?;
            let run = (|| {
                if m.signature.state.is_empty() {
                    let mut args = Vec::with_capacity(m.signature.params.len());
                    for (p, _) in &m.signature.params {
                        args.push(*resolved.get(p).unwrap() as u16); // validated: all params fed
                    }
                    let rep = host.run(handle, &args, &[], budget)?;
                    Ok((rep.result, rep.cycles, rep.trapped_ops, Vec::new()))
                } else {
                    let fields: Vec<(String, u64)> =
                        resolved.iter().map(|(k, v)| (k.clone(), *v)).collect();
                    let (rep, state) = host.run_state(handle, &fields, budget)?;
                    Ok::<_, String>((rep.result, rep.cycles, rep.trapped_ops, state))
                }
            })();
            host.unload(handle)?; // always return the bus to the pool
            let (result, cyc, trap, node_state) = run?;

            // Record this node's outputs for downstream wires.
            values.insert(format!("{node}.result"), result as u64);
            for (f, v) in &node_state {
                values.insert(format!("{node}.{f}"), *v);
            }

            let mut inputs_rec: Vec<(String, u64)> = resolved.into_iter().collect();
            inputs_rec.sort();
            total_cyc += cyc;
            total_trap += trap;
            trace.push(NodeTrace {
                node: node.clone(),
                cell,
                inputs: inputs_rec,
                result,
                cycles: cyc,
                trapped_ops: trap,
            });
        }

        let mut outputs = Vec::with_capacity(self.outputs.len());
        for (name, port) in &self.outputs {
            let v = *values
                .get(&port.key())
                .ok_or_else(|| format!("output `{name}`: `{}` was not produced", port.key()))?;
            outputs.push((name.clone(), v));
        }
        Ok(GraphRun {
            outputs,
            trace,
            cycles: total_cyc,
            trapped_ops: total_trap,
        })
    }
}
