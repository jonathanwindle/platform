// Copyright 2020 Jonathan Windle

// This file is part of Platform.

// Platform is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Platform is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with Platform.  If not, see <https://www.gnu.org/licenses/>.

use crate::irc::BUFFER_SIZE;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::net::{IpAddr, Ipv4Addr};
use std::ops::Add;
use std::str::from_utf8;

pub struct Connection {
    tcp_stream: TcpStream,
}

impl Connection {
    pub fn id(&self) -> String {
        let ip = match self.tcp_stream.peer_addr() {
            Ok(addr) => addr.ip(),
            Err(_e) => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        };
        let port = match self.tcp_stream.peer_addr() {
            Ok(addr) => addr.port(),
            Err(_e) => 0,
        };
        format!("{:}:{:}", ip, port)
    }

    pub fn stream(&self) -> &TcpStream {
        &self.tcp_stream
    }

    pub fn new(tcp_stream: TcpStream) -> Connection {
        Connection {
            tcp_stream: tcp_stream,
        }
    }
}

pub struct Message {
    command: String,
    parameters: Vec<String>,
    prefix: String,
}

impl Message {
    pub fn add_parameter(&mut self, parameter: &str) {
        self.parameters.push(parameter.to_string());
    }

    pub fn command(&self) -> &String {
        &self.command
    }

    pub fn parameters(&self) -> &Vec<String> {
        &self.parameters
    }

    pub fn set_command(&mut self, command: &str) {
        self.command = command.to_string();
    }

    pub fn set_prefix(&mut self, prefix: &str) {
        self.prefix = prefix.to_string();
    }

    pub fn string(&self) -> String {
        let mut string = String::new();
        if !self.prefix.is_empty() {
            string.push_str(":");
            string.push_str(&self.prefix);
            string.push_str(" ");
        }
        string.push_str(&self.command);
        for p in &self.parameters {
            string.push_str(" ");
            if p.contains(" ") {
                string.push_str(":");
                string.push_str(&p);
                break;
            }
            string.push_str(&p);
        }
        string.push_str("\r\n");
        string
    }

    pub fn from_string(string: String) -> Message {
        let mut prefix = String::new();
        let mut command = String::new();
        let mut parameters = Vec::new();
        let mut last_parameter = String::new();

        for (i, p) in string.split(' ').enumerate() {
            if i == 0 {
                if p.chars().nth(0) == Some(':') {
                    let mut p = p.to_string();
                    p.remove(0);
                    prefix = p;
                } else {
                    command = p.to_string();
                }
            } else if i == 1 && !prefix.is_empty() {
                command = p.to_string();
            } else if !last_parameter.is_empty() {
                last_parameter.push_str(p);
                last_parameter.push_str(" ");
            } else if p.chars().nth(0) == Some(':') {
                let mut p = p.to_string();
                p.remove(0);
                last_parameter.push_str(&p);
                last_parameter.push_str(" ");
            } else {
                parameters.push(p.to_string());
            }
        }

        if !last_parameter.is_empty() {
            last_parameter.pop();
            parameters.push(last_parameter);
        }

        Message {
            command: command,
            parameters: parameters,
            prefix: prefix,
        }
    }

    pub fn new() -> Message {
        Message {
            command: String::new(),
            parameters: Vec::new(),
            prefix: String::new(),
        }
    }
}

pub struct Reply {
    messages: Vec<Message>,
}

impl Reply {
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn add_messages(&mut self, messages: &mut Vec<Message>) {
        self.messages.append(messages);
    }

    pub fn mut_messages(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    pub fn strings(&self) -> Result<Vec<String>, ErrorKind> {
        let mut data = Vec::new();
        let mut buffer = String::new();
        for message in &self.messages {
            let string = message.string();
            if string.as_bytes().len() > BUFFER_SIZE {
                return Err(ErrorKind::InvalidData);
            }
            if buffer.as_bytes().len() + string.as_bytes().len() <= BUFFER_SIZE {
                buffer.push_str(&string);
            } else {
                data.push(buffer);
                buffer = message.string();
            }
        }
        if !buffer.is_empty() {
            data.push(buffer);
        }
        Ok(data)
    }

    pub fn new() -> Reply {
        Reply {
            messages: Vec::new(),
        }
    }
}

impl Add for Reply {
    type Output = Self;
    fn add(mut self, mut other: Self) -> Self {
        let mut reply = Reply::new();
        reply.add_messages(self.mut_messages());
        reply.add_messages(other.mut_messages());
        reply
    }
}

pub struct Request {
    data: [u8; BUFFER_SIZE],
    messages: Vec<Message>,
    size: usize,
}

impl Request {
    pub fn clear_data(&mut self) {
        self.data = [0 as u8; BUFFER_SIZE];
        self.messages.clear();
        self.size = 0;
    }

    pub fn data(&mut self) -> &mut [u8; BUFFER_SIZE] {
        &mut self.data
    }

    pub fn messages(&mut self) -> &Vec<Message> {
        if self.messages.is_empty() {
            for message in self.string().split("\r\n") {
                if message != "" {
                    self.messages
                        .push(Message::from_string(message.to_string()));
                }
            }
        }
        &self.messages
    }

    pub fn size(&mut self) -> usize {
        let mut size: usize = 0;

        if self.size == 0 {
            for c in &self.data[..] {
                if *c == 0 {
                    break;
                }
                size = size + 1;
            }

            self.size = size;
        }

        self.size
    }

    pub fn string(&mut self) -> String {
        let size = self.size();
        match from_utf8(&self.data()[..size]) {
            Ok(s) => s.to_string(),
            Err(_e) => "".to_string(),
        }
    }

    pub fn valid(&mut self) -> bool {
        let size = self.size();
        if size > 2 && self.data[size - 1] == b'\n' && self.data[size - 2] == b'\r' {
            true
        } else {
            false
        }
    }

    pub fn new() -> Request {
        Request {
            data: [0 as u8; BUFFER_SIZE],
            messages: Vec::new(),
            size: 0,
        }
    }
}
