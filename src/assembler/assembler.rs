#![allow(unused)]
use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};
use rand::*;
use std::collections::{HashMap, HashSet};
use std::{fs, thread};
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, FloodResponse, Fragment, NodeType};
use wg_2024::packet::{Packet, PacketType};

pub struct Assembler {
    //map every fragment set with it's sessionID
    received_fragment: HashMap<u64, Vec<Fragment>>,
}

impl Assembler {
    pub fn new() -> Self {
        Assembler {
            received_fragment: HashMap::new(),
        }
    }

    //adds the fragment based on the session id and returns it if the message is complete
    pub fn add_fragment(&mut self, fragment: Fragment, session_id: u64) -> Option<Vec<u8>> {
        //check if session id already present or create empty entry with new Vec
        let fragments = self
            .received_fragment
            .entry(session_id)
            .or_insert_with(Vec::new);

        //push the fragment in the right vec
        fragments.push(fragment);

        //if the fragments lenght == field total_n_fragments => reassemble message and return
        if let Some(total_fragment) = fragments.first().map(|f| f.total_n_fragments) {
            if fragments.len() == total_fragment as usize {
                return Some(self.reassemble_message(session_id));
            }
        }
        None
    }

    fn reassemble_message(&mut self, session_id: u64) -> Vec<u8> {
        //take the fragments corresponding to session id
        if let Some(mut fragments) = self.received_fragment.remove(&session_id) {
            //sort the fragment by index
            fragments.sort_by_key(|f| f.fragment_index);

            let mut message = Vec::new();
            for fragment in fragments {
                //add fragment data to message
                message.extend_from_slice(&fragment.data[..fragment.length as usize]);
            }

            return message;
        }

        Vec::new() //to fix
    }
}
