use mio::{Interest, Token};
use std::{
    collections::BTreeMap,
    io::{ErrorKind::WouldBlock, Read},
    net::SocketAddr,
    time::Duration,
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
    tick_duration: Option<Duration>,
    last_token: usize,
    read_buffer: Box<[u8; 4096]>,
    connections: BTreeMap<Token, NetConnection>,
    pending_errors: BTreeMap<Token, NetEvent<'static>>,
}

struct NetConnection {
    socket: mio::net::TcpStream,
    write_buffer: Vec<u8>,
    scheduled_disconnect: bool,
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
            tick_duration: None,
            last_token: 2,
            read_buffer: Box::new([0; 4096]),
            connections: BTreeMap::new(),
            pending_errors: BTreeMap::new(),
        }
    }

    pub fn set_tick(&mut self, tick_duration: Duration) {
        self.tick_duration = Some(tick_duration);
    }

    pub fn receive_event(&mut self) -> (Source, NetEvent<'_>) {
        if let Some(pending_token) = self.pending_errors.keys().next() {
            let token = *pending_token;
            let event = self
                .pending_errors
                .remove(&token)
                .expect("Key checked via iterator");
            return (Source(token.0), event);
        }

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
                                self.connections.insert(
                                    new_token,
                                    NetConnection {
                                        socket: tcp_stream,
                                        write_buffer: Vec::new(),
                                        scheduled_disconnect: false,
                                    },
                                );
                                break (
                                    Source(token.0),
                                    NetEvent::Accepted(Source(new_token.0), socket_addr),
                                );
                            }
                            Err(e) if e.kind() == WouldBlock => {
                                self.ready_sockets.pop();
                                continue;
                            }
                            Err(_e) => panic!("Sad..."),
                        }
                    } else {
                        let stream = &mut self
                            .connections
                            .get_mut(&token)
                            .expect("Unregistered token")
                            .socket;

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
                    let token = *token;
                    match self.write_bytes(&token) {
                        Some(event) => break (Source(token.0), event),
                        None => continue,
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

    fn disconnect(&mut self, token: Token) {
        self.connections
            .remove(&token)
            .expect("Invalid source provided");
        self.ready_sockets.retain(|t| t.0 != token);
    }

    pub fn schedule_disconnect(&mut self, target: &Source) {
        let token = Token(target.0);

        if self.pending_errors.get(&token).is_some() {
            // It's already pending a different kind of disconnect.
            return;
        }

        let connection = self
            .connections
            .get_mut(&token)
            .expect("Invalid source provided");

        connection.scheduled_disconnect = true;

        self.mio_poll
            .registry()
            .reregister(
                &mut connection.socket,
                token,
                Interest::READABLE | Interest::WRITABLE,
            )
            .unwrap();
    }

    fn write_bytes(&mut self, token: &Token) -> Option<NetEvent<'static>> {
        let connection = self.connections.get_mut(token).expect("Unregistered token");
        let stream = &mut connection.socket;
        let write_buffer = &mut connection.write_buffer;

        match std::io::Write::write(stream, &*write_buffer) {
            Ok(bytes) => {
                assert_ne!(bytes, 0, "Wrote zero bytes?");

                write_buffer.drain(..bytes);

                if write_buffer.is_empty() {
                    let token = *token;
                    self.mio_poll
                        .registry()
                        .reregister(stream, token, Interest::READABLE)
                        .unwrap();
                    self.ready_sockets
                        .retain(|(t, ready)| *t != token || !matches!(ready, Ready::Writable));

                    if connection.scheduled_disconnect {
                        self.mio_poll.registry().deregister(stream).unwrap();
                        self.disconnect(token);
                        return Some(NetEvent::Disconnected);
                    }
                }
            }
            Err(e) if e.kind() == WouldBlock => {
                self.ready_sockets.pop();
            }
            Err(_e) => panic!("This is even sadder..."),
        }

        None
    }

    /// ### Panics
    /// Panics when an invalid target is provided.
    pub fn send_bytes(&mut self, target: &Source, bytes: &[u8]) {
        let token = Token(target.0);

        // Check if the token is still valid; it may no longer exist if there
        // was an error during a flush, and the Disconnect event was not yet
        // sent.
        if self.pending_errors.get(&token).is_some() {
            return;
        }

        let connection = self
            .connections
            .get_mut(&token)
            .expect("Invalid source provided");

        connection.write_buffer.extend_from_slice(bytes);
        self.mio_poll
            .registry()
            .reregister(
                &mut connection.socket,
                token,
                Interest::READABLE | Interest::WRITABLE,
            )
            .unwrap();
    }

    /// ### Panics
    /// Panics when an invalid target is provided
    pub fn try_flush(&mut self, target: &Source) {
        let token = Token(target.0);

        if self.pending_errors.get(&token).is_some() {
            return;
        }

        if let Some(event) = self.write_bytes(&token) {
            self.pending_errors.insert(token, event);
        };
    }
}
