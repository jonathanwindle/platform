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

extern crate num_cpus;

mod irc;

fn main() {
    let mut listener = irc::Listener::new();
    let service = irc::Service::new();
    for _ in 0..num_cpus::get() {
        let worker = irc::Worker::new(listener.clone_request_queue(), service.clone());
        let _ = worker.run();
    }
    listener.set_bind_string("127.0.0.1:6667".to_string());
    let t = listener.run();
    let _ = t.join();
}
