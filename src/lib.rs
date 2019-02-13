#[macro_use]
extern crate serde_derive;

use redis::Commands;
use rmp_serde::Serializer;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Emitter {
    redis: redis::Client,
    prefix: String,
    nsp: String,
    channel: String,
    rooms: Vec<String>,
    flags: HashMap<String, bool>,
    uid: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Opts {
    rooms: Vec<String>,
    flags: HashMap<String, bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Packet {
    #[serde(rename = "type")]
    _type: i32,
    data: Vec<String>,
    nsp: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EmitterOpts<'a> {
    host: String,
    port: i32,
    socket: Option<String>,
    key: Option<&'a str>,
}

pub trait New<T> {
    fn new(_: T) -> Emitter;
}

impl New<redis::Client> for Emitter {
    fn new(redis: redis::Client) -> Emitter {
        Emitter::emitter(redis, "socket.io", "/")
    }
}

impl<'a> New<EmitterOpts<'a>> for Emitter {
    fn new(opts: EmitterOpts) -> Emitter {
        let addr = format!("redis://{}:{}", opts.host, opts.port);
        let prefix = match opts.key {
            Some(key) => key,
            None => "socket.io",
        };

        Emitter::emitter(redis::Client::open(addr.as_str()).unwrap(), prefix, "/")
    }
}

impl New<&str> for Emitter {
    fn new(addr: &str) -> Emitter {
        Emitter::emitter(
            redis::Client::open(format!("redis://{}", addr).as_str()).unwrap(),
            "socket.io",
            "/",
        )
    }
}

impl Emitter {
    fn emitter(redis: redis::Client, prefix: &str, nsp: &str) -> Emitter {
        Emitter {
            redis: redis,
            prefix: prefix.to_string(),
            nsp: nsp.to_string(),
            channel: format!("{}#{}#", prefix, nsp),
            rooms: Vec::new(),
            flags: HashMap::new(),
            uid: "emitter".to_string(),
        }
    }
    pub fn to(mut self, room: &str) -> Emitter {
        self.rooms.push(room.to_string());
        self
    }
    pub fn of(self, nsp: &str) -> Emitter {
        Emitter::emitter(self.redis, self.prefix.as_str(), nsp)
    }
    pub fn json(mut self) -> Emitter {
        let mut flags = HashMap::new();
        flags.insert("json".to_string(), true);
        self.flags = flags;
        self
    }
    pub fn volatile(mut self) -> Emitter {
        let mut flags = HashMap::new();
        flags.insert("volatile".to_string(), true);
        self.flags = flags;
        self
    }
    pub fn broadcast(mut self) -> Emitter {
        let mut flags = HashMap::new();
        flags.insert("broadcast".to_string(), true);
        self.flags = flags;
        self
    }
    pub fn emit(mut self, message: Vec<&str>) -> Emitter {
        let packet = Packet {
            _type: 2,
            data: message.iter().map(|s| s.to_string()).collect(),
            nsp: self.nsp.clone(),
        };
        let opts = Opts {
            rooms: self.rooms.clone(),
            flags: self.flags.clone(),
        };
        let mut msg = Vec::new();
        let val = (self.uid.clone(), packet, opts);
        val.serialize(&mut Serializer::new_named(&mut msg)).unwrap();

        let channel = if self.rooms.len() == 1 {
            format!("{}{}", self.channel.clone(), self.rooms.join("#"))
        } else {
            self.channel.clone()
        };
        let _: () = self.redis.publish(channel, msg).unwrap();
        self.rooms = vec![];
        self.flags = HashMap::new();
        self
    }
}
