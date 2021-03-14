use mio::{Interest, Token};
use std::{collections::BTreeMap, time::Duration};
use std::{
    io::{ErrorKind::WouldBlock, Read},
    net::SocketAddr,
};

#[repr(transparent)]
#[derive(Debug)]
pub struct Source(pub usize);

#[derive(Debug)]
pub enum NetEvent<'a> {
    Accepted(Source, SocketAddr),
    Disconnected,
    Received(&'a [u8]),
    Tick,
    // Event,
}

pub struct NetServer {
    mio_poll: mio::Poll,
    mio_events: mio::Events,
    ready_sockets: Vec<(Token, Ready)>,
    listener: mio::net::TcpListener,
    sockets: BTreeMap<Token, mio::net::TcpStream>,
    tick_duration: Option<Duration>,
    last_token: usize,
    read_buffer: Box<[u8; 4096]>,
    write_buffers: BTreeMap<Token, Vec<u8>>,
}

pub enum Ready {
    Readable,
    Writable,
}

impl NetServer {
    pub fn new(addr: SocketAddr) -> Self {
        let mio_poll = mio::Poll::new().unwrap();

        let mut listener = mio::net::TcpListener::bind(addr).unwrap();

        mio_poll
            .registry()
            .register(&mut listener, Token(1), Interest::READABLE)
            .unwrap();

        NetServer {
            mio_poll,
            mio_events: mio::Events::with_capacity(128),
            ready_sockets: Vec::with_capacity(128),
            listener,
            sockets: BTreeMap::new(),
            tick_duration: None,
            last_token: 2,
            read_buffer: Box::new([0; 4096]),
            write_buffers: BTreeMap::new(),
        }
    }

    pub fn set_tick(&mut self, tick_duration: Duration) {
        self.tick_duration = Some(tick_duration);
    }

    pub fn receive_event(&mut self) -> (Source, NetEvent<'_>) {
        // TODO: Check interrupt
        loop {
            match self.ready_sockets.first() {
                Some((token, Ready::Readable)) => {
                    if *token == Token(1) {
                        match self.listener.accept() {
                            Ok((mut tcp_stream, socket_addr)) => {
                                let new_token = Token(self.last_token);
                                self.last_token += 1;
                                self.mio_poll
                                    .registry()
                                    .register(&mut tcp_stream, new_token, Interest::READABLE)
                                    .unwrap();
                                self.sockets.insert(new_token, tcp_stream);
                                self.write_buffers.insert(new_token, Vec::new());
                                break (Source(token.0), NetEvent::Accepted(Source(new_token.0), socket_addr));
                            }
                            Err(e) if e.kind() == WouldBlock => {
                                self.ready_sockets.pop();
                                continue;
                            }
                            Err(_e) => panic!("Sad..."),
                        }
                    } else {
                        let stream = self.sockets.get_mut(&token).expect("Unregistered token");

                        match stream.read(self.read_buffer.as_mut()) {
                            Ok(bytes) if bytes == 0 => {
                                let token = *token;
                                self.mio_poll.registry().deregister(stream).unwrap();
                                self.disconnect(token);
                                break (Source(token.0), NetEvent::Disconnected);
                            }
                            Ok(bytes) => {
                                break (
                                    Source(token.0),
                                    NetEvent::Received(&self.read_buffer[..bytes]),
                                );
                            }
                            Err(e) if e.kind() == WouldBlock => {
                                self.ready_sockets.pop();
                                continue;
                            }
                            Err(error) => {
                                println!("Socket error: {}", error);
                                let token = *token;
                                self.mio_poll.registry().deregister(stream).unwrap();
                                self.disconnect(token);
                                break (Source(token.0), NetEvent::Disconnected);
                            }
                        }
                    }
                }
                Some((token, Ready::Writable)) => {
                    let stream = self.sockets.get_mut(token).expect("Unregistered token");
                    let write_buffer = self.write_buffers.get_mut(token).expect("Unregistered token");

                    match std::io::Write::write(stream, &*write_buffer) {
                        Ok(bytes) => {
                            assert_ne!(bytes, 0, "Wrote zero bytes?");

                            write_buffer.drain(..bytes);

                            if write_buffer.is_empty() {
                                let token = *token;
                                self.mio_poll.registry().reregister(stream, token, Interest::READABLE).unwrap();
                                self.ready_sockets.retain(|(t, ready)| *t != token || !matches!(ready, Ready::Writable));
                            }
                        }
                        Err(e) if e.kind() == WouldBlock => {
                            self.ready_sockets.pop();
                            continue;
                        }
                        Err(_e) => panic!("This is even sadder..."),
                    }

                }
                None => {
                    self.mio_poll
                        .poll(&mut self.mio_events, self.tick_duration)
                        .unwrap();
                    if self.mio_events.is_empty() {
                        break (Source(0), NetEvent::Tick);
                    } else {
                        for event in &self.mio_events {
                            if event.is_readable() {
                                self.ready_sockets.push((event.token(), Ready::Readable));
                            }
                            if event.is_writable() {
                                self.ready_sockets.push((event.token(), Ready::Writable));
                            }
                        }
                        continue;
                    }
                }
            }
        }
    }

    pub fn disconnect(&mut self, token: Token) {
        self.sockets
            .remove(&token)
            .expect("Was looking at it a second ago");
        self.ready_sockets.retain(|t| t.0 != token);
        self.write_buffers.remove(&token);
    }

    /// ### Panics
    /// Panics when an invalid target is provided.
    pub fn send_bytes(&mut self, target: &Source, bytes: &[u8]) {
        let token = Token(target.0);
        let write_buffer = self
            .write_buffers
            .get_mut(&token)
            .expect("Invalid source provided");
        let socket = self
            .sockets
            .get_mut(&token)
            .expect("Invalid source provided");

        write_buffer.extend_from_slice(bytes);
        self.mio_poll
            .registry()
            .reregister(socket, token, Interest::READABLE | Interest::WRITABLE)
            .unwrap();
    }
}
