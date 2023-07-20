// Stract is an open source web search engine.
// Copyright (C) 2023 Stract ApS
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{cmp::Reverse, collections::BinaryHeap, sync::Arc};

use crate::{
    intmap::{IntMap, IntSet},
    ranking::inbound_similarity::InboundSimilarity,
    webgraph::{Node, NodeID, Webgraph},
};

const MAX_SIMILAR_SITES: usize = 1_000;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ScoredNode {
    pub node: Node,
    pub score: f64,
}

struct ScoredNodeID {
    node_id: NodeID,
    score: f64,
}

impl PartialOrd for ScoredNodeID {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl PartialEq for ScoredNodeID {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Ord for ScoredNodeID {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Eq for ScoredNodeID {}

pub struct SimilarSitesFinder {
    webgraph: Arc<Webgraph>,
    inbound_similarity: InboundSimilarity,
}

impl SimilarSitesFinder {
    pub fn new(webgraph: Arc<Webgraph>, inbound_similarity: InboundSimilarity) -> Self {
        Self {
            webgraph,
            inbound_similarity,
        }
    }

    pub fn find_similar_sites(&self, nodes: &[String], limit: usize) -> Vec<ScoredNode> {
        let limit = limit.min(MAX_SIMILAR_SITES);

        let nodes: Vec<_> = nodes
            .iter()
            .map(|url| Node::from(url.to_string()).into_host())
            .map(|node| node.id())
            .collect();

        let scorer = self.inbound_similarity.scorer(&nodes, &[]);

        let mut backlink_count: IntMap<NodeID, usize> = IntMap::new();

        for node in &nodes {
            for edge in self.webgraph.raw_ingoing_edges(node) {
                if !backlink_count.contains_key(&edge.from) {
                    backlink_count.insert(edge.from, 0);
                }

                *backlink_count.get_mut(&edge.from).unwrap() += 1;
            }
        }

        let mut top_backlink_nodes: Vec<_> = backlink_count
            .into_iter()
            .map(|(node, count)| (node, count))
            .collect();

        top_backlink_nodes.sort_unstable_by_key(|(_, count)| *count);
        top_backlink_nodes.reverse();

        let mut potential_nodes = IntSet::new();
        for (backlink_node, _) in top_backlink_nodes {
            for edge in self.webgraph.raw_outgoing_edges(&backlink_node) {
                potential_nodes.insert(edge.to);
            }
        }

        let mut scored_nodes = BinaryHeap::with_capacity(limit);

        for potential_node in potential_nodes.into_iter() {
            let score = scorer.score(&potential_node);
            let scored_node_id = ScoredNodeID {
                node_id: potential_node,
                score,
            };

            if scored_nodes.len() < limit {
                scored_nodes.push(Reverse(scored_node_id));
            } else {
                let mut min_scored_node = scored_nodes.peek_mut().unwrap();

                if scored_node_id > min_scored_node.0 {
                    *min_scored_node = Reverse(scored_node_id);
                }
            }
        }

        let mut scored_nodes: Vec<_> = scored_nodes.into_iter().take(limit).map(|n| n.0).collect();
        scored_nodes.sort();
        scored_nodes.reverse();

        scored_nodes
            .into_iter()
            .map(|ScoredNodeID { node_id, score }| {
                let node = self.webgraph.id2node(&node_id).unwrap();
                ScoredNode { node, score }
            })
            .collect()
    }

    pub fn knows_about(&self, node: &Node) -> bool {
        self.inbound_similarity.knows_about(node.id())
    }
}
