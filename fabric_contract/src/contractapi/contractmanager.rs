/*
 * SPDX-License-Identifier: Apache-2.0
 */
#![allow(dead_code)]
#![allow(unused_imports)]
use crate::contractapi::context::*;
use crate::contractapi::contract::*;
use crate::{dataapi::WireBuffer, contractapi::contractdefn, contract::ContractError};
use fabric_ledger_protos::common_messages;

use lazy_static::lazy_static;

use log::{debug, trace, warn, info};

use std::collections::HashMap;
use std::sync::Mutex;

// Static reference to the ContractManager
lazy_static! {
    static ref CONTRACT_MGR: Mutex<ContractManager> = Mutex::new(ContractManager::new());
}

/// Contract Manager structure that holds the list contract objects
pub struct ContractManager {
    contracts: HashMap<String, contractdefn::ContractDefn>,
    // TODO current channel/transaction ID almost certainly doesn't belong here!
    channelid: String,
    transactionid: String,
}

impl ContractManager {
    pub fn new() -> ContractManager {
        ContractManager {
            contracts: HashMap::new(),
            // TODO so horrible
            channelid: String::from(""),
            transactionid: String::from(""),
        }
    }

    // TODO don't do this!
    fn create_context(self: &ContractManager) -> common_messages::TransactionContext {
        let mut tx_context = common_messages::TransactionContext::new();
        tx_context.set_channel_id(self.channelid.clone());
        tx_context.set_transaction_id(self.transactionid.clone());
        tx_context
    }

    fn register_contract_impl(self: &mut ContractManager, contract: Box<dyn Contract + Send>) {
        let name = contract.name();
        
        let contract_defn = contractdefn::ContractDefn::new(contract);
        

        self.contracts.insert(name, contract_defn);
    }

    fn evaluate(
        self: &mut ContractManager,
        ctx: &mut Context,
        contract_name: String,
        tx: String,
        args: &[Vec<u8>],
        transient: &[Vec<u8>]
    ) -> Result<WireBuffer, ContractError> {
        debug!("contractmanager::evaluate {} {}", contract_name, tx);

        match self.contracts.get(&contract_name) {
            Some(defn) => {
                // TODO do something more sensible with the context!!!
                self.channelid = ctx.get_channelid().to_string();
                self.transactionid = ctx.get_txid().to_string();
                let r = defn.invoke(ctx,tx,args/*,transient*/);
                r
            }
            None => {
                warn!(
                    "Unable to find contract Failed {}.{},{:?}",
                    contract_name, tx, args
                );
                Err(ContractError::from(String::from("Unable to find contract")))
            }
        }
    }

    /// register the contract
    pub fn register_contract(contract: Box<dyn Contract + Send>) {
        CONTRACT_MGR
            .lock()
            .unwrap()
            .register_contract_impl(contract);
    }

    /// Route the call to the correct contract
    pub fn route(ctx: &mut Context, tx: String, args: &[Vec<u8>], transient: &[Vec<u8>]) -> Result<WireBuffer, ContractError> {
        trace!("contractmanager::route>>");

        // parse out the contract_name here
        let namespace: String;
        let fn_name: String;
        match tx.find(":") {
            None => {
                namespace = "default".to_string();
                fn_name = tx.clone();
            }
            Some(s) => {
                namespace = tx[..s].to_string();
                fn_name = tx[s + 1..].to_string();
            }
        }

        let r = CONTRACT_MGR
            .lock()
            .unwrap()
            .evaluate(ctx, namespace, fn_name, args,transient);

        trace!("contractmanager::route<<");
        r
    }

    // TODO don't do this!
    pub fn get_context() -> common_messages::TransactionContext {
        let tx_context = CONTRACT_MGR
            .lock()
            .unwrap()
            .create_context();

        tx_context
    }
}
