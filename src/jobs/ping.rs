// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ping {
    pub greeting: String,
    pub reply: Option<String>,
}

impl Ping {
    pub fn new(greeting: &str) -> Self {
        Self {
            greeting: greeting.to_owned(),
            reply: None,
        }
    }
}

pub async fn run(mut ping: Ping) -> Result<Ping> {
    ping.reply = Some(format!("You said '{}', I say 'PONG'!", ping.greeting));
    Ok(ping)
}
